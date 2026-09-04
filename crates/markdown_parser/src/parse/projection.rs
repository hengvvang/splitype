//! Document-level block projection with incremental re-parse.
//!
//! A [`BlockProjection`] is the parsed block tree of a document, together
//! with the span of every top-level parse region. The buffer keeps one
//! projection per document and updates it synchronously with each edit:
//! [`BlockProjection::reparse`] re-parses only the window around the lines
//! the edit touched — expanding it to the enclosing block constructs on
//! both sides — and splices the result into the previous tree. Per-edit
//! cost is O(edited region), independent of document size, and the result
//! is structurally identical to a full parse: blocks outside the edited
//! window keep their identity, so view entities survive projection patches.
//!
//! The projection reads lines directly from the shared [`Rope`]: no
//! full-text materialization or revision diff happens on the edit path.

use std::sync::Arc;

use rope::Rope;

use super::code_and_text::{collect_comment_block, collect_indented_code_block};
use super::helpers::*;
use super::lists::collect_list_blocks;
use super::pipeline::{ParseMode, RegionSpan, build_blocks_from_lines_with_regions};
use super::quotes::collect_quote_block;
use crate::block::image::parse_standalone_image;
use crate::block::table::{
    collect_pipeless_table_region, collect_table_candidate_region, is_table_candidate_line,
    parse_table_region,
};
use crate::parse::data::BlockData;
use crate::parse::indent::{is_quote_start, strip_indented_code_prefix};
use crate::parse::kind::BlockKind;
use crate::parse::lines::{Lines, RopeLines};

/// The parsed block tree of a document, together with the region layout
/// that makes re-parsing incremental.
#[derive(Clone, Debug)]
pub struct BlockProjection {
    /// The block tree in DFS order (roots interleaved with their children).
    pub blocks: Arc<Vec<BlockData>>,
    /// Line and flat-list spans of every top-level parse region, sorted by
    /// line. Private: the layout is re-parse infrastructure, not content.
    regions: Vec<RegionSpan>,
}

impl BlockProjection {
    /// Full parse of `rope` in Linewise (editing) mode with the region
    /// layout recorded. Lines are read straight from the rope: O(lines)
    /// lookups, no text materialization.
    pub fn parse(rope: &Rope) -> BlockProjection {
        let lines = RopeLines::new(rope);
        let (blocks, regions) =
            build_blocks_from_lines_with_regions(&lines, ParseMode::Linewise, true);
        BlockProjection {
            blocks: Arc::new(blocks),
            regions,
        }
    }

    /// Incremental re-parse in place: re-parses only the window around the
    /// changed lines and splices the result into this projection.
    /// Structurally identical to [`BlockProjection::parse`] of the same
    /// rope; blocks outside the edited window keep their identity so view
    /// entities survive projection patches. When no snapshot shares the
    /// block list (the normal editing case), the splice is in-place — the
    /// untouched tail is moved, not cloned — so per-edit cost is
    /// O(edited region).
    ///
    /// `first_changed`/`last_changed` are the edited lines in the document
    /// coordinates the projection was last parsed from (inclusive).
    pub fn reparse(
        projection: &mut BlockProjection,
        rope: &Rope,
        first_changed: usize,
        last_changed: usize,
    ) {
        reparse(projection, rope, first_changed, last_changed);
    }

    /// Index of the region containing `line` (0-based source line).
    fn region_index_of(&self, line: usize) -> usize {
        self.regions.partition_point(|r| r.line_start <= line) - 1
    }

    /// Whether the region is a single blank line (an empty paragraph block).
    /// Blank regions are transparent to block constructs: a loose list or
    /// indented code run can absorb them together with the lines after them.
    fn is_blank_region(&self, index: usize) -> bool {
        region_is_blank(&self.blocks, &self.regions, index)
    }
}

fn reparse(
    projection: &mut BlockProjection,
    rope: &Rope,
    first_changed: usize,
    last_changed: usize,
) {
    // No layout to patch against (empty document): full parse.
    if projection.regions.is_empty() {
        *projection = BlockProjection::parse(rope);
        return;
    }

    let old_line_count = projection
        .regions
        .last()
        .expect("non-empty region layout")
        .line_end;
    let first_changed_line = first_changed.min(old_line_count - 1);
    let last_changed_line = last_changed.min(old_line_count - 1).max(first_changed_line);

    // Direct rope access: every line read is an O(log m) chunk lookup, so
    // no line list and no full-text materialization exists on this path.
    let new_lines = RopeLines::new(rope);
    let new_line_count = new_lines.line_count();
    let delta = new_line_count as isize - old_line_count as isize;

    // 2. Initial window: the whole old regions containing the changed
    //    lines. Both boundaries sit on old region edges; the fixpoint
    //    loops below keep that invariant.
    let mut window_start =
        projection.regions[projection.region_index_of(first_changed_line)].line_start;
    let mut window_end = projection.regions[projection.region_index_of(last_changed_line)].line_end;

    // 3. Backward fixpoint: while the construct ending at window_start
    //    extends past it in the new text (an edit can turn the window's
    //    first line into a continuation, a fence content line, a setext
    //    underline, ...), pull the window start back to that construct's
    //    beginning. Blank regions are transparent: the absorbing construct
    //    may sit behind a run of blanks (loose lists, indented code).
    let mut steps = 0usize;
    while window_start > 0 {
        steps += 1;
        if steps > 256 {
            *projection = BlockProjection::parse(rope);
            return;
        }
        let mut probe = projection.region_index_of(window_start - 1);
        while probe > 0 && projection.is_blank_region(probe) {
            probe -= 1;
        }
        let extent = construct_extent(&new_lines, projection.regions[probe].line_start);
        if extent > window_start {
            window_start = projection.regions[probe].line_start;
            continue;
        }
        break;
    }

    // 4. Forward fixpoint: parse the window; while its last construct
    //    extends past the window end in the new text (a fence opened at the
    //    window tail, a list or quote growing into the following lines, a
    //    paragraph turned setext heading), grow the window to the
    //    construct's full extent — rounded up to whole old regions — and
    //    re-parse. Each iteration jumps a full construct, so this loop is
    //    short even when the edit opens a document-spanning fence.
    let mut window_blocks = Vec::new();
    let mut window_regions = Vec::new();
    let mut stable = false;
    for _ in 0..64 {
        let new_window_end = ((window_end as isize) + delta).max(window_start as isize) as usize;
        let window = new_lines.slice(window_start, new_window_end);
        let (blocks, regions) =
            build_blocks_from_lines_with_regions(&window, ParseMode::Linewise, true);

        // The construct that could extend past the window is the last
        // non-blank region: trailing blanks may be absorbed together with
        // lines beyond the window (loose lists, indented code).
        let mut probe = regions.len();
        while probe > 0 && region_is_blank(&blocks, &regions, probe - 1) {
            probe -= 1;
        }
        if probe == 0 {
            window_blocks = blocks;
            window_regions = regions;
            stable = true;
            break;
        }
        let absolute_start = window_start + regions[probe - 1].line_start;
        let extent = construct_extent(&new_lines, absolute_start);
        if extent <= new_window_end {
            window_blocks = blocks;
            window_regions = regions;
            stable = true;
            break;
        }
        let new_end_in_old = (extent as isize - delta).max(1) as usize;
        let enclosing =
            projection.region_index_of(new_end_in_old.saturating_sub(1).min(old_line_count - 1));
        window_end = projection.regions[enclosing].line_end.max(window_end + 1);
    }
    if !stable {
        *projection = BlockProjection::parse(rope);
        return;
    }

    splice(
        projection,
        delta,
        window_start,
        window_end,
        window_blocks,
        window_regions,
    );
}

/// Replaces the old regions covering `[window_start, window_end)` with the
/// re-parsed window blocks. Everything before keeps its ids and spans;
/// everything after keeps its ids and shifts by `delta` lines. The block
/// list is spliced in place when it is uniquely owned (memmove of the tail),
/// and rebuilt by copying when outstanding snapshots share it.
#[allow(clippy::too_many_arguments)]
fn splice(
    projection: &mut BlockProjection,
    delta: isize,
    window_start: usize,
    window_end: usize,
    window_blocks: Vec<BlockData>,
    window_regions: Vec<RegionSpan>,
) {
    let ri_start = projection
        .regions
        .partition_point(|r| r.line_start < window_start);
    let ri_end = projection
        .regions
        .partition_point(|r| r.line_end <= window_end)
        - 1;

    let old_flat_start = projection.regions[ri_start].flat_start;
    let old_flat_end = projection.regions[ri_end].flat_end;
    let window_len = window_blocks.len();

    if let Some(blocks) = Arc::get_mut(&mut projection.blocks) {
        blocks.splice(old_flat_start..old_flat_end, window_blocks);
    } else {
        let mut blocks = Vec::with_capacity(
            old_flat_start + window_len + (projection.blocks.len() - old_flat_end),
        );
        blocks.extend_from_slice(&projection.blocks[..old_flat_start]);
        blocks.extend(window_blocks);
        blocks.extend_from_slice(&projection.blocks[old_flat_end..]);
        projection.blocks = Arc::new(blocks);
    }

    let flat_shift = (old_flat_start + window_len) as isize - old_flat_end as isize;

    let mut regions = Vec::with_capacity(
        ri_start + window_regions.len() + (projection.regions.len() - ri_end - 1),
    );
    regions.extend_from_slice(&projection.regions[..ri_start]);
    for region in &window_regions {
        regions.push(RegionSpan {
            line_start: region.line_start + window_start,
            line_end: region.line_end + window_start,
            flat_start: region.flat_start + old_flat_start,
            flat_end: region.flat_end + old_flat_start,
        });
    }
    for region in &projection.regions[ri_end + 1..] {
        regions.push(RegionSpan {
            line_start: (region.line_start as isize + delta) as usize,
            line_end: (region.line_end as isize + delta) as usize,
            flat_start: (region.flat_start as isize + flat_shift) as usize,
            flat_end: (region.flat_end as isize + flat_shift) as usize,
        });
    }

    projection.regions = regions;
}

/// The extent (exclusive end line) of the top-level construct starting at
/// `start` in `lines`, mirroring the pipeline's dispatch order exactly.
/// This is the boundary oracle of the incremental re-parse: a window edge is
/// stable iff the construct on either side ends exactly at it.
fn construct_extent<L: Lines + ?Sized>(lines: &L, start: usize) -> usize {
    let Some(line) = lines.get(start) else {
        return start;
    };
    if line.trim().is_empty() {
        return start + 1;
    }
    if let Some(fence) = parse_opening_fence(line) {
        return match find_matching_closing_fence(lines, start, &fence) {
            Some(close) => close + 1,
            None => lines.line_count(),
        };
    }
    if let Some((_, end)) = collect_comment_block(lines, start) {
        return end;
    }
    if is_block_html_start(line) {
        return collect_block_html_region(lines, start);
    }
    if is_footnote_definition_start(line) {
        return collect_footnote_definition_region(lines, start);
    }
    if is_reference_definition_start(line) {
        return collect_reference_definition_region(lines, start);
    }
    if parse_standalone_image(line).is_some() {
        return start + 1;
    }
    if strip_indented_code_prefix(line).is_some() {
        if let Some((_, end)) = collect_indented_code_block(lines, start) {
            return end;
        }
        return start + 1;
    }
    if parse_list_marker(line).is_some() {
        return collect_list_blocks(lines, start).1;
    }
    if is_quote_start(line) {
        return collect_quote_block(lines, start).1;
    }
    if BlockKind::parse_atx_heading_line(line).is_some() {
        return start + 1;
    }
    if BlockKind::parse_thematic_break_line(line) {
        return start + 1;
    }
    if is_table_candidate_line(line) {
        // The pipeline consumes the whole candidate region either way: a
        // successful parse becomes one table block, a failed one becomes
        // one paragraph block per line — the region span is the same.
        return collect_table_candidate_region(lines, start);
    }
    if let Some(end) = collect_pipeless_table_region(lines, start)
        && parse_table_region(&lines.slice(start, end)).is_some()
    {
        return end;
    }
    if is_display_math_start(line) {
        return collect_display_math_region(lines, start);
    }
    if let Some(next) = lines.get(start + 1)
        && parse_list_marker(next).is_none()
        && BlockKind::parse_setext_underline(next).is_some()
    {
        return start + 2;
    }
    start + 1
}

/// Whether a parsed region is a single blank line (an empty paragraph
/// block). Blank regions are transparent to block constructs that can
/// absorb them together with following lines.
fn region_is_blank(blocks: &[BlockData], regions: &[RegionSpan], index: usize) -> bool {
    let region = &regions[index];
    region.line_end == region.line_start + 1
        && blocks
            .get(region.flat_start)
            .is_some_and(|block| block.kind == BlockKind::Paragraph && block.text.plain_len() == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::data::blocks_content_eq;

    /// The changed line range in old coordinates, derived from the byte
    /// diff the way the buffer derives it from its edit ranges.
    fn changed_lines(old: &str, new: &str) -> (usize, usize) {
        let (prefix, suffix) = crate::parse::common_affix(old, new);
        let changed_start = prefix;
        let changed_end = old.len() - suffix;
        let line_of = |byte: usize| {
            old.as_bytes()[..byte.min(old.len())]
                .iter()
                .filter(|&&b| b == b'\n')
                .count()
        };
        let old_line_count = old.lines().count().max(1);
        let first = line_of(changed_start).min(old_line_count - 1);
        let last = if changed_end > changed_start {
            // `changed_end` may sit exactly on a line start (a deleted
            // newline merges its line into the next one); the line
            // starting there is affected too.
            line_of(changed_end)
                .min(old_line_count - 1)
                .max(line_of(changed_end - 1))
        } else {
            first
        };
        (first, last)
    }

    /// Every construct-extent answer must agree with the pipeline: parsing
    /// from a line and looking at the first region's span gives the same
    /// extent the boundary oracle reports. Exhaustively checked per line of
    /// a construct-heavy corpus.
    #[test]
    fn construct_extent_matches_pipeline() {
        let corpus = [
            "",
            "plain paragraph",
            "# Heading\nbody",
            "text\n===\nbody",
            "text\n---\nbody",
            "```rust\ncode\n```\nafter",
            "```\nunclosed",
            "- a\n- b\n  - c\n\nafter",
            "- a\n\n  cont\n\nafter",
            "> quote\n> more\n\nafter",
            "    indented\n    code\n\nafter",
            "| a | b |\n| - | - |\n| 1 | 2 |\nafter",
            "a | b\n- | -\nrow one\nafter",
            "$$ math $$\nafter",
            "<!-- comment -->\nafter",
            "<div>\nhtml\n</div>\nafter",
            "[^note]: footnote\n    continuation\nafter",
            "[ref]: https://example.com\n    title\nafter",
            "![alt](img.png)\nafter",
            "***\nafter",
            "内容段落\nafter",
            "1. one\n2. two\nafter",
        ];
        for text in corpus {
            let lines: Vec<&str> = text.lines().collect();
            for start in 0..lines.len() {
                // The pipeline reports the first region's span relative to
                // the slice; construct_extent reports an absolute end.
                let expected = build_blocks_from_lines_with_regions(
                    &lines[start..],
                    ParseMode::Linewise,
                    true,
                )
                .1
                .first()
                .map_or(lines.len(), |region| start + region.line_end);
                assert_eq!(
                    construct_extent(&lines[..], start),
                    expected,
                    "construct_extent diverged for {text:?} at line {start}"
                );
            }
        }
    }

    /// One incremental re-parse after one small edit equals a full parse.
    fn check_reparse(old: &str, new: &str) {
        let old_rope = Rope::new(old);
        let new_rope = Rope::new(new);
        let mut projection = BlockProjection::parse(&old_rope);
        let (first, last) = changed_lines(old, new);
        BlockProjection::reparse(&mut projection, &new_rope, first, last);
        let full = BlockProjection::parse(&new_rope);
        assert!(
            blocks_content_eq(&projection.blocks, &full.blocks),
            "incremental diverged from full parse:\nold: {old:?}\nnew: {new:?}"
        );
    }

    #[test]
    fn reparses_typed_text_inside_paragraph() {
        check_reparse("# Title\n\nBody text.\n", "# Title\n\nBody text!.\n");
    }

    #[test]
    fn reparses_typing_at_end_of_document() {
        check_reparse("one\ntwo", "one\ntwo\nthree");
    }

    #[test]
    fn reparses_insertion_at_start() {
        check_reparse("first\nsecond", "inserted\nfirst\nsecond");
    }

    #[test]
    fn reparses_setext_creation_and_removal() {
        check_reparse("text\nplain", "text\n===");
        check_reparse("text\n===", "text\nplain");
        check_reparse("text\n---\nnext", "text\n---\n===");
    }

    #[test]
    fn reparses_fence_opening_and_closing() {
        check_reparse("para\ncode\nafter", "para\n```\ncode\nafter");
        check_reparse("para\n```\ncode\n```\nafter", "para\n```\ncode\nafter");
        check_reparse("para\n```\ncode\n```\nafter", "para\nx\ncode\nx\nafter");
    }

    #[test]
    fn reparses_list_continuation_through_blanks() {
        check_reparse("- a\n\nb\n", "- a\n\n  cont\n");
        check_reparse("- a\n\nb\n\nc", "- a\n\n  x\n\nc");
        check_reparse("- a\n- b\n\nc", "- a\n- b\nc");
    }

    #[test]
    fn reparses_quote_and_indented_code_merges() {
        check_reparse("> a\nplain", "> a\n> b");
        check_reparse("> a\n\nplain", "> a\n\n> b");
        check_reparse("    code\nplain", "    code\n    more");
        check_reparse("    code\n\nplain", "    code\n\n    more");
    }

    #[test]
    fn reparses_tables_and_rows() {
        check_reparse(
            "| a | b |\n| - | - |\n| 1 | 2 |\nplain",
            "| a | b |\n| - | - |\n| 1 | 2 |\n| 3 | 4 |",
        );
        check_reparse("| a |\n| b |\nplain", "| a |\n| b |\n| c |");
        check_reparse("a | b\n- | -\nrow\nafter", "a | b\n- | -\nrow\nextra");
    }

    #[test]
    fn reparses_thematic_break_and_heading_edges() {
        check_reparse("para\n***\nnext", "para\n---\nnext");
        check_reparse("para\n---\nnext", "para\n***\nnext");
        check_reparse("## H\nbody", "## H2\nbody");
    }

    #[test]
    fn reparses_deletions_and_joins() {
        check_reparse("keep\nremove\nkeep", "keep\nkeep");
        check_reparse("a\n\nb\n\nc", "a\nb\nc");
        check_reparse("- a\n\n- b", "- a\n- b");
        check_reparse("- a\n- b", "- a\n\n- b");
    }

    #[test]
    fn reparses_to_empty_and_from_empty() {
        check_reparse("some text", "");
        check_reparse("", "new text");
        check_reparse("", "");
    }

    #[test]
    fn reparses_multibyte_content() {
        check_reparse("第一行\n第二行\n", "第一行内容\n第二行\n");
        check_reparse("内容\n", "内\n容\n");
        check_reparse("前\n中文段落\n后", "前\n中文段落。\n后");
    }

    #[test]
    fn reparses_footnote_and_math_regions() {
        check_reparse("[^a]: note\n\nbody", "[^a]: note\n    cont\n\nbody");
        check_reparse("$$ math $$\nbody", "$$ math $$\nmore");
        check_reparse("para\n$$ open\nbody", "para\n$$ open\n$$");
    }

    #[test]
    fn preserved_regions_keep_their_ids() {
        let old_rope = Rope::new("# A\n\nbody\n\n# C");
        let new_rope = Rope::new("# A\n\nchanged\n\n# C");
        let mut projection = BlockProjection::parse(&old_rope);
        let before_ids: Vec<_> = projection
            .blocks
            .iter()
            .map(|block| (block.id, block.kind.clone()))
            .collect();
        let (first, last) = changed_lines("# A\n\nbody\n\n# C", "# A\n\nchanged\n\n# C");
        BlockProjection::reparse(&mut projection, &new_rope, first, last);
        assert_eq!(
            projection.blocks[0].id, before_ids[0].0,
            "leading unchanged blocks must keep their ids"
        );
        assert_eq!(
            projection.blocks[3].id, before_ids[3].0,
            "trailing unchanged blocks must keep their ids"
        );
        let full = BlockProjection::parse(&new_rope);
        assert!(blocks_content_eq(&projection.blocks, &full.blocks));
    }

    #[test]
    fn shared_block_list_is_rebuilt_by_copy_without_mutating_snapshots() {
        let old_rope = Rope::new("# A\n\nbody\n\n# C");
        let new_rope = Rope::new("# A\n\nchanged\n\n# C");
        let mut projection = BlockProjection::parse(&old_rope);
        // A pane snapshot keeps the old block list alive; the re-parse must
        // not mutate it in place.
        let snapshot_blocks = projection.blocks.clone();
        let (first, last) = changed_lines("# A\n\nbody\n\n# C", "# A\n\nchanged\n\n# C");
        BlockProjection::reparse(&mut projection, &new_rope, first, last);
        assert_eq!(snapshot_blocks.len(), 5);
        // The snapshot still carries the old content, untouched.
        let old_full = BlockProjection::parse(&old_rope);
        assert!(blocks_content_eq(&snapshot_blocks, &old_full.blocks));
        let full = BlockProjection::parse(&new_rope);
        assert!(blocks_content_eq(&projection.blocks, &full.blocks));
    }

    /// Deterministic pseudo-random fuzz: a construct-heavy document receives
    /// many random edits; after every one, the incremental re-parse must be
    /// structurally identical to a full parse of the edited text.
    #[test]
    fn random_edits_always_match_full_parse() {
        struct Lcg(u64);
        impl Lcg {
            fn next(&mut self) -> u64 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                self.0 >> 33
            }
        }
        let mut rng = Lcg(0x9e3779b97f4a7c15);

        let pieces = [
            "# 标题\n",
            "plain 内容\n",
            "- item\n",
            "  - nested\n",
            "> quote\n",
            "```rust\ncode line\n```\n",
            "| a | b |\n| - | - |\n| 1 | 2 |\n",
            "$$ math $$\n",
            "\n",
            "***\n",
            "1. numbered\n",
            "text\n===\n",
            "[^n]: note\n",
            "<!-- comment -->\n",
        ];
        let mut text = String::from("# 开始\n");
        for i in 0..40 {
            text.push_str(pieces[(rng.next() as usize) % pieces.len()]);
            if i % 5 == 0 {
                text.push('\n');
            }
        }

        let old_rope = Rope::new(&text);
        let mut projection = BlockProjection::parse(&old_rope);
        for _ in 0..120 {
            let mut edited = text.clone();
            let edits = 1 + (rng.next() % 3) as usize;
            for _ in 0..edits {
                let mut at = (rng.next() as usize) % (edited.len().max(1));
                while !edited.is_char_boundary(at) {
                    at -= 1;
                }
                let len = (rng.next() as usize) % 24;
                let mut end = (at + len).min(edited.len());
                while !edited.is_char_boundary(end) {
                    end -= 1;
                }
                let insert = [
                    "x", "\n", "```\n", "- item\n", "---\n", "> q\n", "内容", "", " ",
                ][(rng.next() as usize) % 9];
                edited.replace_range(at..end, insert);
            }
            let new_rope = Rope::new(&edited);
            let (first, last) = changed_lines(&text, &edited);
            BlockProjection::reparse(&mut projection, &new_rope, first, last);
            let full = BlockProjection::parse(&new_rope);
            assert!(
                blocks_content_eq(&projection.blocks, &full.blocks),
                "incremental diverged from full parse.\nbefore: {text:?}\nafter:  {edited:?}"
            );
            text = edited;
        }
    }
}
