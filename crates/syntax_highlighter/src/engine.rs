//! Native incremental highlighting engine, mirroring Zed's `SyntaxMap`.
//!
//! One [`HighlightMap`] owns the persistent tree-sitter parse trees for a
//! document: the root language tree plus one tree per injected language
//! layer (Markdown inline markup, fenced-code languages, HTML blocks).
//! Edits reuse trees via tree-sitter's `Tree::edit`, and spans resolve inner
//! layers over outer layers.
//!
//! The map works purely in document byte offsets — like Zed's syntax map it
//! keeps no line table of its own; the buffer's rope owns line knowledge.
//! Highlights are a flat, sorted span list; consumers slice ranges out of
//! it with their own line mapping.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use tree_sitter::{InputEdit, Parser, Point, Query, QueryCursor, StreamingIterator, Tree};

use rope::Rope;

use crate::highlight::{
    CodeHighlightSpan, class_for_highlight, language_config, resolve_code_language_key,
};
use crate::language::CodeLanguageKey;

/// A language's grammar and queries. Configs are `Send + Sync` (the grammar
/// is a function), so they live in a process-global registry; each
/// [`HighlightMap`] builds its own `Query` instances at construction.
pub struct LanguageConfig {
    pub name: &'static str,
    /// Builds the tree-sitter language value.
    pub grammar: fn() -> tree_sitter::Language,
    pub highlights_query: &'static str,
    pub injections_query: &'static str,
}

/// One injected language layer: the document byte ranges it covers plus the
/// parse tree covering exactly those ranges.
#[derive(Clone)]
struct Layer {
    key: CodeLanguageKey,
    query: Arc<Query>,
    injection_query: Option<Arc<Query>>,
    tree: Tree,
    /// Document byte ranges this layer covers, sorted, chunked so an edit
    /// touches O(chunk) ranges and shifts every following chunk by one base
    /// adjustment — apply_edit is O(edit + chunks), never O(ranges).
    chunks: RangeChunks,
}

/// Incremental highlight state for one document. The map does not own the
/// text: callers (the document buffer) pass its rope to [`apply_edit`] and
/// [`refresh`].
///
/// [`apply_edit`]: HighlightMap::apply_edit
/// [`refresh`]: HighlightMap::refresh
#[derive(Clone)]
pub struct HighlightMap {
    config: Arc<LanguageConfig>,
    query: Arc<Query>,
    injection_query: Option<Arc<Query>>,
    /// Root-language parse tree.
    tree: Tree,
    /// Injection layers, outer to inner.
    layers: Vec<Layer>,
    /// Flat, sorted, non-overlapping spans in document coordinates,
    /// computed by the last refresh.
    spans: Arc<[CodeHighlightSpan]>,
    /// Bumped on every edit; consumers compare it to detect stale data.
    pub version: u64,
    /// The version the spans were computed for.
    pub refreshed_version: u64,
    /// Byte range (new-text coordinates) awaiting refresh.
    dirty: Option<Range<usize>>,
    /// Union span of dropped layer ranges (old coordinates), for
    /// re-discovering injections that start before the dirty range.
    dropped_layers: Option<Range<usize>>,
}

impl HighlightMap {
    /// Full parse of `text` as `key`, including injections. Returns `None`
    /// for languages without a tree-sitter configuration (callers fall back
    /// to light rules).
    pub fn new(key: CodeLanguageKey, text: &str) -> Option<Self> {
        let config = language_config(key)?;
        let query = Arc::new(Query::new(&(config.grammar)(), config.highlights_query).ok()?);
        let injection_query = (!config.injections_query.is_empty())
            .then(|| Query::new(&(config.grammar)(), config.injections_query).ok())
            .flatten()
            .map(Arc::new);

        let rope = Rope::new(text);
        let mut map = Self {
            config: config.clone(),
            query,
            injection_query,
            tree: parse(&(config.grammar)(), text, &[], &rope, None),
            layers: Vec::new(),
            spans: Arc::from([]),
            version: 0,
            refreshed_version: 0,
            dirty: None,
            dropped_layers: None,
        };
        map.refresh(&rope);
        Some(map)
    }

    /// Records an edit into the map's internal state. `rope` is the text
    /// before the edit; `range` is replaced by `inserted`. Cheap: O(edit)
    /// tree edits + O(edit + chunks) range maintenance. The span list is
    /// deliberately left alone: it is rebuilt by the next
    /// [`HighlightMap::refresh`], and every consumer gates on the refresh
    /// version (stale-while-revalidate), so maintaining offset-correct
    /// spans between refreshes would be pure waste on the keystroke path.
    pub fn apply_edit(&mut self, rope: &Rope, range: Range<usize>, inserted: &str) {
        self.version = self.version.wrapping_add(1);
        let start = range.start.min(rope.len());
        let end = range.end.min(rope.len()).max(start);
        let delta = inserted.len() as isize - (end - start) as isize;

        // Root tree: record the edit; the re-parse happens on refresh.
        let edit = InputEdit {
            start_byte: start,
            old_end_byte: end,
            new_end_byte: start + inserted.len(),
            start_position: ts_point(rope, start),
            old_end_position: ts_point(rope, end),
            new_end_position: point_plus(ts_point(rope, end), inserted),
        };
        self.tree.edit(&edit);

        // Injection layers: shift ranges after the edit, drop ranges it
        // intersects (re-discovered on refresh), remove emptied layers.
        let mut dropped: Option<Range<usize>> = None;
        let mut idx = 0;
        while idx < self.layers.len() {
            self.layers[idx]
                .chunks
                .apply_edit(start, end, delta, &mut dropped);
            self.layers[idx].tree.edit(&edit);
            if self.layers[idx].chunks.is_empty() {
                self.layers.remove(idx);
            } else {
                idx += 1;
            }
        }
        self.dropped_layers = match (self.dropped_layers.take(), dropped) {
            (Some(a), Some(b)) => Some(a.start.min(b.start)..a.end.max(b.end)),
            (a, b) => a.or(b),
        };

        // Merge the dirty region.
        let dirty = start..start + inserted.len();
        self.dirty = Some(match self.dirty.take() {
            Some(prev) => prev.start.min(dirty.start)..prev.end.max(dirty.end),
            None => dirty,
        });
    }

    /// Re-parses dirty regions and recomputes the span list. Incremental:
    /// the edited trees are reused, so unchanged regions cost nothing; the
    /// queries themselves run over the whole document, exactly like Zed's
    /// syntax map reparse.
    pub fn refresh(&mut self, rope: &Rope) {
        let dirty = self.dirty.take();
        let dropped = self.dropped_layers.take();
        let text = rope.materialize();

        // 1. Root tree: incremental re-parse (tree-sitter only re-parses
        //    what the edits invalidated).
        self.tree = parse(&(self.config.grammar)(), &text, &[], rope, Some(&self.tree));

        // 2. Re-discover injections over the recover region: the dirty range
        //    plus dropped layer spans (their starts precede the dirty range,
        //    e.g. a fence edited from inside). No edits: refresh everything.
        let recover = match (dirty, dropped) {
            (None, None) => 0..text.len(),
            (Some(dirty), Some(dropped)) => {
                dirty.start.min(dropped.start)..dirty.end.max(dropped.end)
            }
            (Some(dirty), None) => dirty,
            (None, Some(dropped)) => dropped,
        };
        let recover = recover.start.min(text.len())..recover.end.min(text.len());

        let mut discovered: HashMap<CodeLanguageKey, Vec<Range<usize>>> = HashMap::new();
        self.discover_layers(&text, recover.clone(), &mut discovered);
        self.add_layers(&text, rope, discovered, recover, 0);

        // 3. Recompute all spans: root query overlaid by layer queries,
        //    inner layers winning over outer layers.
        let mut spans = collect_spans(&self.query, &self.tree, &text, 0..text.len());
        for layer in &self.layers {
            let layer_spans = collect_spans(&layer.query, &layer.tree, &text, 0..text.len());
            spans = overlay_spans(spans, layer_spans);
        }
        self.spans = Arc::from(spans);
        self.refreshed_version = self.version;
    }

    /// The flat span list, in document coordinates.
    pub fn spans(&self) -> &[CodeHighlightSpan] {
        &self.spans
    }

    /// The shared span list (reference-counted; snapshots hand it to panes
    /// without copying).
    pub fn spans_arc(&self) -> Arc<[CodeHighlightSpan]> {
        self.spans.clone()
    }

    /// All spans intersecting `range`, in document coordinates.
    pub fn spans_in_range(&self, range: Range<usize>) -> Vec<CodeHighlightSpan> {
        let start_idx = self
            .spans
            .partition_point(|span| span.range.end <= range.start);
        let mut out = Vec::new();
        for span in &self.spans[start_idx..] {
            if span.range.start >= range.end {
                break;
            }
            let start = span.range.start.max(range.start);
            let end = span.range.end.min(range.end);
            if start < end {
                out.push(CodeHighlightSpan {
                    range: start..end,
                    class: span.class,
                });
            }
        }
        out
    }

    /// Flattens all spans into one sorted list (one-shot consumers).
    pub fn into_flat_spans(self) -> Vec<CodeHighlightSpan> {
        self.spans.to_vec()
    }

    /// Adopts the refreshed spans of another map, but only when no newer
    /// edit has landed meanwhile (version match). Returns whether the
    /// refresh was adopted.
    pub fn adopt_refresh(&mut self, map: HighlightMap) -> bool {
        if self.version != map.version {
            return false;
        }
        self.spans = map.spans;
        self.refreshed_version = map.refreshed_version;
        true
    }

    // ── Internals ─────────────────────────────────────────────────────────

    /// Adds or merges discovered injection layers, re-parsing affected
    /// ranges (old trees reused), then recurses into the nested injections
    /// of the affected layers up to a small depth.
    fn add_layers(
        &mut self,
        text: &str,
        rope: &Rope,
        discovered: HashMap<CodeLanguageKey, Vec<Range<usize>>>,
        recover: Range<usize>,
        depth: usize,
    ) {
        const MAX_DEPTH: usize = 4;
        if depth > MAX_DEPTH {
            return;
        }
        let affected: Vec<CodeLanguageKey> = discovered.keys().copied().collect();
        for (key, mut new_ranges) in discovered {
            let Some(config) = language_config(key) else {
                continue;
            };
            new_ranges.sort_by_key(|r| r.start);
            new_ranges.dedup_by(|a, b| a.start == b.start && a.end == b.end);
            if let Some(layer) = self.layers.iter_mut().find(|layer| layer.key == key) {
                let mut ranges = layer.chunks.materialize();
                ranges.retain(|r| r.start >= recover.end || r.end <= recover.start);
                ranges.extend(new_ranges);
                if !ranges.is_empty() {
                    let chunks = RangeChunks::from_sorted(ranges);
                    let included = chunks.materialize();
                    layer.tree = parse(
                        &(config.grammar)(),
                        text,
                        &included,
                        rope,
                        Some(&layer.tree),
                    );
                    layer.chunks = chunks;
                }
            } else {
                let Some(query) = Query::new(&(config.grammar)(), config.highlights_query).ok()
                else {
                    continue;
                };
                let injection_query = (!config.injections_query.is_empty())
                    .then(|| Query::new(&(config.grammar)(), config.injections_query).ok())
                    .flatten()
                    .map(Arc::new);
                let tree = parse(&(config.grammar)(), text, &new_ranges, rope, None);
                self.layers.push(Layer {
                    key,
                    query: Arc::new(query),
                    injection_query,
                    tree,
                    chunks: RangeChunks::from_sorted(new_ranges),
                });
            }
        }
        self.layers.retain(|layer| !layer.chunks.is_empty());

        // Nested injections of the affected layers, restricted to their
        // ranges inside the recover region.
        let mut nested: HashMap<CodeLanguageKey, Vec<Range<usize>>> = HashMap::new();
        for key in affected {
            let Some(layer) = self.layers.iter().find(|layer| layer.key == key) else {
                continue;
            };
            let Some(injection_query) = layer.injection_query.as_ref() else {
                continue;
            };
            let ranges: Vec<Range<usize>> = layer
                .chunks
                .materialize()
                .into_iter()
                .filter(|r| r.start < recover.end && r.end > recover.start)
                .collect();
            if ranges.is_empty() {
                continue;
            }
            collect_injections(injection_query, &layer.tree, text, &ranges, &mut nested);
        }
        nested.retain(|inner_key, _| !self.layers.iter().any(|layer| layer.key == *inner_key));
        if !nested.is_empty() {
            self.add_layers(text, rope, nested, recover, depth + 1);
        }
    }

    /// Collects injections of the root tree within `range` into `found`.
    fn discover_layers(
        &self,
        text: &str,
        range: Range<usize>,
        found: &mut HashMap<CodeLanguageKey, Vec<Range<usize>>>,
    ) {
        let Some(injection_query) = &self.injection_query else {
            return;
        };
        collect_injections(
            injection_query,
            &self.tree,
            text,
            std::slice::from_ref(&range),
            found,
        );
    }
}

/// Sorted document byte ranges of one injection layer, stored in fixed-size
/// chunks with per-chunk coordinate bases: an edit rebuilds only the chunk
/// it touches and shifts every following chunk by one base adjustment, so
/// per-edit cost is O(edit + chunks) instead of O(ranges). Refresh paths
/// materialize the absolute list in O(ranges).
#[derive(Clone, Default)]
struct RangeChunks {
    /// Sorted by absolute coordinates; a chunk's stored ranges are relative
    /// to its base (absolute = stored + base).
    chunks: Vec<RangeChunk>,
}

const RANGE_CHUNK_TARGET: usize = 4096;
const RANGE_CHUNK_MAX: usize = RANGE_CHUNK_TARGET * 2;
/// Coalescing threshold for nearly adjacent injection ranges.
const RANGE_MERGE_GAP: usize = 8;

#[derive(Clone)]
struct RangeChunk {
    /// Coordinate offset: absolute = stored + base. Stored coordinates are
    /// always non-negative (base <= the chunk's first range start).
    base: isize,
    /// Sorted, non-overlapping ranges in coordinates relative to `base`.
    ranges: Vec<Range<usize>>,
}

impl RangeChunks {
    fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    fn len(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.ranges.len()).sum()
    }

    /// The absolute range list (refresh paths only; O(ranges)).
    fn materialize(&self) -> Vec<Range<usize>> {
        let mut out = Vec::with_capacity(self.len());
        for chunk in &self.chunks {
            out.extend(
                chunk
                    .ranges
                    .iter()
                    .map(|range| shifted_range(range, chunk.base)),
            );
        }
        out
    }

    /// Re-chunks a sorted absolute range list, first coalescing nearly
    /// adjacent ranges (gaps up to a few separator bytes — e.g. the blank
    /// line between blocks) so the layer tree carries far fewer included
    /// ranges: tree-sitter's per-edit cost scales with them.
    fn from_sorted(mut ranges: Vec<Range<usize>>) -> Self {
        ranges.sort_by_key(|range| range.start);
        ranges.dedup_by(|a, b| a.start == b.start && a.end == b.end);
        let mut coalesced: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
        for range in ranges {
            if let Some(previous) = coalesced.last_mut()
                && range.start.saturating_sub(previous.end) <= RANGE_MERGE_GAP
            {
                previous.end = previous.end.max(range.end);
            } else {
                coalesced.push(range);
            }
        }
        Self::chunked(coalesced)
    }

    /// Chunks an already-sorted, non-overlapping absolute range list
    /// without coalescing (used by the re-chunk guard, which must preserve
    /// the exact range set).
    fn chunked(ranges: Vec<Range<usize>>) -> Self {
        let mut chunks = Vec::with_capacity(ranges.len() / RANGE_CHUNK_TARGET + 1);
        for group in ranges.chunks(RANGE_CHUNK_TARGET) {
            let base = group[0].start as isize;
            chunks.push(RangeChunk {
                base,
                ranges: group
                    .iter()
                    .map(|range| shifted_range(range, -base))
                    .collect(),
            });
        }
        Self { chunks }
    }

    /// Applies one edit: drops ranges intersecting `[start, end)` (their
    /// absolute span is unioned into `dropped`), leaves the earlier ones
    /// alone, and shifts the later ones by `delta` through their chunks'
    /// bases.
    fn apply_edit(
        &mut self,
        start: usize,
        end: usize,
        delta: isize,
        dropped: &mut Option<Range<usize>>,
    ) {
        // First chunk whose absolute end exceeds `start`.
        let first = self
            .chunks
            .partition_point(|chunk| chunk.absolute_end() <= start);
        let mut index = first;
        let mut rebuilt = Vec::new();
        while index < self.chunks.len() {
            let chunk = &self.chunks[index];
            if chunk.absolute_start() >= end {
                break;
            }
            let mut prefix = Vec::new();
            let mut suffix = Vec::new();
            for range in &chunk.ranges {
                let absolute = shifted_range(range, chunk.base);
                if absolute.end <= start {
                    prefix.push(range.clone());
                } else if absolute.start >= end {
                    // Stored coordinates are base-relative, so the shifted
                    // range keeps the same storage under base + delta.
                    suffix.push(range.clone());
                } else {
                    union_range(dropped, absolute);
                }
            }
            if !prefix.is_empty() {
                rebuilt.push(RangeChunk {
                    base: chunk.base,
                    ranges: prefix,
                });
            }
            if !suffix.is_empty() {
                rebuilt.push(RangeChunk {
                    base: chunk.base + delta,
                    ranges: suffix,
                });
            }
            index += 1;
        }
        for chunk in &mut self.chunks[index..] {
            chunk.base += delta;
        }
        self.chunks.splice(first..index, rebuilt);
        self.merge();
        // Guard against pathological chunk growth: re-chunk when the count
        // drifts past the target density (amortized O(ranges), preserving
        // the exact range set).
        let ideal = self.len() / RANGE_CHUNK_TARGET + 1;
        if self.chunks.len() > ideal * 2 {
            *self = Self::chunked(self.materialize());
        }
    }

    /// Coalesces adjacent chunks sharing a base, bounding the chunk count.
    fn merge(&mut self) {
        let mut merged: Vec<RangeChunk> = Vec::with_capacity(self.chunks.len());
        for chunk in self.chunks.drain(..) {
            if let Some(previous) = merged.last_mut()
                && previous.base == chunk.base
                && previous.ranges.len() + chunk.ranges.len() <= RANGE_CHUNK_MAX
            {
                previous.ranges.extend(chunk.ranges);
            } else {
                merged.push(chunk);
            }
        }
        self.chunks = merged;
    }
}

impl RangeChunk {
    fn absolute_start(&self) -> usize {
        self.ranges
            .first()
            .map(|range| shift_usize(range.start, self.base))
            .unwrap_or(0)
    }

    fn absolute_end(&self) -> usize {
        self.ranges
            .last()
            .map(|range| shift_usize(range.end, self.base))
            .unwrap_or(0)
    }
}

fn shifted_range(range: &Range<usize>, delta: isize) -> Range<usize> {
    shift_usize(range.start, delta)..shift_usize(range.end, delta)
}

fn union_range(union: &mut Option<Range<usize>>, range: Range<usize>) {
    match union {
        Some(existing) => {
            existing.start = existing.start.min(range.start);
            existing.end = existing.end.max(range.end);
        }
        None => *union = Some(range),
    }
}

#[cfg(test)]
mod chunked_range_tests {
    use super::*;

    #[test]
    fn chunked_ranges_match_flat_maintenance() {
        // A reference flat list maintained the old way must equal the
        // chunked store's materialization after every edit, including
        // multi-chunk documents and the dropped-range union.
        let mut reference: Vec<Range<usize>> = (0..20_000).map(|i| i * 20..i * 20 + 4).collect();
        // Gaps exceed the coalescing threshold so the reference model and
        // the chunked store start from identical content.
        let mut chunked = RangeChunks::from_sorted(reference.clone());
        let edits: &[(usize, usize, &str)] = &[
            (50_000, 50_000, "x"),   // insert before a range
            (50_001, 50_003, ""),    // delete inside a range
            (0, 0, "\n\n"),          // insert at the start
            (399_990, 400_004, "z"), // replace the tail
            (60_000, 120_000, ""),   // delete across many ranges
            (30_000, 30_000, "y"),   // insert inside a gap
            (100_200, 100_204, ""),  // delete one range exactly
        ];
        for (start, end, inserted) in edits {
            let start = *start;
            let end = *end;
            let delta = inserted.len() as isize - (end - start) as isize;

            let mut expected_dropped: Option<Range<usize>> = None;
            let mut next: Vec<Range<usize>> = Vec::with_capacity(reference.len());
            for range in &reference {
                if range.end <= start {
                    next.push(range.clone());
                } else if range.start >= end {
                    next.push(shift_usize(range.start, delta)..shift_usize(range.end, delta));
                } else {
                    match &mut expected_dropped {
                        Some(existing) => {
                            existing.start = existing.start.min(range.start);
                            existing.end = existing.end.max(range.end);
                        }
                        None => expected_dropped = Some(range.clone()),
                    }
                }
            }
            let mut actual_dropped = None;
            chunked.apply_edit(start, end, delta, &mut actual_dropped);
            assert_eq!(
                chunked.materialize(),
                next,
                "edit ({start}..{end}, {inserted:?}) diverged"
            );
            assert_eq!(
                actual_dropped, expected_dropped,
                "dropped union diverged for edit ({start}..{end}, {inserted:?})"
            );
            reference = next;
        }
    }

    /// Randomized fuzz of the chunked range maintenance against the flat
    /// reference model.
    #[test]
    fn random_edits_keep_chunked_ranges_consistent() {
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
        let mut rng = Lcg(0x9e37_79b9_7f4a_7c15);
        let mut reference: Vec<Range<usize>> = Vec::new();
        let mut cursor = 0usize;
        for _ in 0..12_000 {
            cursor += 12 + (rng.next() as usize) % 7;
            reference.push(cursor..cursor + 1 + (rng.next() as usize) % 5);
            cursor += 12 + (rng.next() as usize) % 5;
        }
        let mut chunked = RangeChunks::from_sorted(reference.clone());
        for step in 0..400 {
            let len = reference.last().map(|r| r.end).unwrap_or(0);
            let at = (rng.next() as usize) % (len + 1);
            let removed = (rng.next() as usize) % 30;
            let end = (at + removed).min(len);
            let inserted = ["", "x", "\n", "content", "中"][(rng.next() as usize) % 5];
            let delta = inserted.len() as isize - (end - at) as isize;

            let mut expected_dropped: Option<Range<usize>> = None;
            let mut next: Vec<Range<usize>> = Vec::with_capacity(reference.len());
            for range in &reference {
                if range.end <= at {
                    next.push(range.clone());
                } else if range.start >= end {
                    next.push(shift_usize(range.start, delta)..shift_usize(range.end, delta));
                } else {
                    match &mut expected_dropped {
                        Some(existing) => {
                            existing.start = existing.start.min(range.start);
                            existing.end = existing.end.max(range.end);
                        }
                        None => expected_dropped = Some(range.clone()),
                    }
                }
            }
            let mut actual_dropped = None;
            chunked.apply_edit(at, end, delta, &mut actual_dropped);
            let actual = chunked.materialize();
            if actual != next {
                let mismatch = actual
                    .iter()
                    .zip(next.iter())
                    .position(|(a, b)| a != b)
                    .or_else(|| Some(actual.len().min(next.len())));
                panic!(
                    "step {step}: edit ({at}..{end}, {inserted:?}) diverged at {mismatch:?}\nactual:   {:?}\nexpected: {:?}",
                    actual.get(mismatch.unwrap_or(0)),
                    next.get(mismatch.unwrap_or(0)),
                );
            }
            assert_eq!(
                actual_dropped, expected_dropped,
                "step {step}: dropped union diverged"
            );
            reference = next;
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Parses `text` (restricted to `ranges` when non-empty), reusing `old_tree`
/// so unchanged regions are not re-parsed.
fn parse(
    language: &tree_sitter::Language,
    text: &str,
    ranges: &[Range<usize>],
    rope: &Rope,
    old_tree: Option<&Tree>,
) -> Tree {
    let mut parser = Parser::new();
    parser.set_language(language).ok();
    if !ranges.is_empty() {
        let ts_ranges: Vec<tree_sitter::Range> = ranges
            .iter()
            .map(|range| tree_sitter::Range {
                start_byte: range.start,
                end_byte: range.end,
                start_point: ts_point(rope, range.start),
                end_point: ts_point(rope, range.end),
            })
            .collect();
        parser.set_included_ranges(&ts_ranges).ok();
    }
    parser.parse(text.as_bytes(), old_tree).unwrap_or_else(|| {
        parser
            .parse(text.as_bytes(), None)
            .expect("parse without old tree")
    })
}

/// The tree-sitter point of a byte offset: rows are newline counts and the
/// column is the byte distance from the line start. Offsets pointing at a
/// newline resolve to the start of the next line, and the end of a
/// newline-terminated document resolves to its trailing line — matching
/// tree-sitter's own point convention. Offsets inside a multi-byte
/// character resolve to the character's start (points are only hints for
/// tree-sitter; the byte offsets stay authoritative).
fn ts_point(rope: &Rope, offset: usize) -> Point {
    let offset = offset.min(rope.len());
    if rope.char_after(offset) == Some('\n') {
        return Point::new(rope.offset_to_point(offset).0 + 1, 0);
    }
    if offset == rope.len() && rope.ends_with_newline() {
        return Point::new(rope.offset_to_point(offset).0 + 1, 0);
    }
    let (row, col) = rope.offset_to_point(offset);
    Point::new(row, col)
}

/// `point` advanced by `text`: rows increase per newline, the column resets
/// (tree-sitter's `new_end_position` of an insertion).
fn point_plus(point: Point, text: &str) -> Point {
    let mut row = point.row;
    let mut col = point.column;
    for byte in text.bytes() {
        if byte == b'\n' {
            row += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Point::new(row, col)
}

fn shift_usize(offset: usize, delta: isize) -> usize {
    if delta >= 0 {
        offset + delta as usize
    } else {
        offset.saturating_sub((-delta) as usize)
    }
}

/// Clamps a byte range outward to the nearest UTF-8 character boundaries.
/// tree-sitter's byte-level lexers can report node edges inside multi-byte
/// characters (notably tree-sitter-md); every range that later feeds string
/// slicing or tree-sitter includes must be boundary-aligned.
fn clamp_to_char_boundaries(text: &str, range: Range<usize>) -> Range<usize> {
    let mut start = range.start.min(text.len());
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = range.end.min(text.len());
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    start..end
}

/// Collects injections (language, content range) from a layer's tree over
/// `ranges`.
fn collect_injections(
    injection_query: &Query,
    tree: &Tree,
    text: &str,
    ranges: &[Range<usize>],
    found: &mut HashMap<CodeLanguageKey, Vec<Range<usize>>>,
) {
    let mut cursor = QueryCursor::new();
    for range in ranges {
        cursor.set_byte_range(range.clone());
        let mut matches = cursor.matches(injection_query, tree.root_node(), text.as_bytes());
        while let Some(m) = matches.next() {
            let mut language: Option<String> = None;
            let mut content: Option<Range<usize>> = None;
            for capture in m.captures {
                let name = injection_query.capture_names()[capture.index as usize];
                match name {
                    "injection.language" => {
                        let range = clamp_to_char_boundaries(text, capture.node.byte_range());
                        language = Some(text[range].to_string());
                    }
                    "injection.content" => {
                        content = Some(clamp_to_char_boundaries(text, capture.node.byte_range()));
                    }
                    _ => {}
                }
            }
            if language.is_none() {
                for property in injection_query.property_settings(m.pattern_index) {
                    if property.key.as_ref() == "injection.language" {
                        language = property.value.as_ref().map(|value| value.to_string());
                    }
                }
            }
            let (Some(language), Some(content)) = (language, content) else {
                continue;
            };
            let Some(key) = resolve_code_language_key(Some(&language)) else {
                continue;
            };
            found.entry(key).or_default().push(content);
        }
    }
}

/// Collects highlight spans of one layer over a byte range, resolving
/// overlapping captures (the most specific — shortest — capture wins).
fn collect_spans(
    query: &Query,
    tree: &Tree,
    text: &str,
    range: Range<usize>,
) -> Vec<CodeHighlightSpan> {
    let mut captures: Vec<CodeHighlightSpan> = Vec::new();
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range);
    let mut matches = cursor.matches(query, tree.root_node(), text.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let name = query.capture_names()[capture.index as usize];
            if let Some(class) = class_for_highlight(name) {
                captures.push(CodeHighlightSpan {
                    range: clamp_to_char_boundaries(text, capture.node.byte_range()),
                    class,
                });
            }
        }
    }
    resolve_overlaps(captures)
}

/// Sorts captures and drops overlaps, keeping the most specific (shortest)
/// span at each position. Deterministic: for equal lengths the later
/// capture wins.
fn resolve_overlaps(mut spans: Vec<CodeHighlightSpan>) -> Vec<CodeHighlightSpan> {
    spans.sort_by_key(|span| (span.range.start, span.range.len()));
    let mut resolved = Vec::new();
    let mut active: Vec<&CodeHighlightSpan> = Vec::new();
    let mut position = 0usize;
    let mut index = 0usize;
    while index < spans.len() || !active.is_empty() {
        let next_start = spans.get(index).map(|span| span.range.start);
        let next_end = active.iter().map(|span| span.range.end).min();
        let event = match (next_start, next_end) {
            (Some(start), Some(end)) => start.min(end),
            (Some(start), None) => start,
            (None, Some(end)) => end,
            (None, None) => break,
        };
        if event > position {
            if let Some(top) = active.iter().min_by_key(|span| span.range.len()) {
                resolved.push(CodeHighlightSpan {
                    range: position..event,
                    class: top.class,
                });
            }
            position = event;
        }
        while index < spans.len() && spans[index].range.start == event {
            active.push(&spans[index]);
            index += 1;
        }
        active.retain(|span| span.range.end > event);
    }
    resolved
}

/// Overlays inner spans on outer spans: inner wins where present, outer
/// fills the gaps. Both lists are sorted and non-overlapping.
fn overlay_spans(
    mut base: Vec<CodeHighlightSpan>,
    overlay: Vec<CodeHighlightSpan>,
) -> Vec<CodeHighlightSpan> {
    let mut merged = Vec::with_capacity(base.len() + overlay.len());
    let mut base_idx = 0usize;
    for span in overlay {
        while base_idx < base.len() && base[base_idx].range.end <= span.range.start {
            merged.push(base[base_idx].clone());
            base_idx += 1;
        }
        if base_idx < base.len() && base[base_idx].range.start < span.range.start {
            let head = CodeHighlightSpan {
                range: base[base_idx].range.start..span.range.start,
                class: base[base_idx].class,
            };
            if head.range.start < head.range.end {
                merged.push(head);
            }
            base[base_idx].range.start = span.range.start;
        }
        if span.range.start < span.range.end {
            merged.push(span.clone());
        }
        while base_idx < base.len() && base[base_idx].range.start < span.range.end {
            if base[base_idx].range.end <= span.range.end {
                base_idx += 1;
            } else {
                base[base_idx].range.start = span.range.end;
                break;
            }
        }
    }
    while base_idx < base.len() {
        merged.push(base[base_idx].clone());
        base_idx += 1;
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::{CodeHighlightClass, highlight_code_block};

    /// An incrementally-edited map must equal a freshly-built map.
    fn assert_edit_equivalence(
        language: CodeLanguageKey,
        base: &str,
        range: Range<usize>,
        inserted: &str,
    ) {
        let mut edited = String::with_capacity(base.len() + inserted.len());
        edited.push_str(&base[..range.start]);
        edited.push_str(inserted);
        edited.push_str(&base[range.end..]);

        let base_rope = Rope::new(base);
        let mut map = HighlightMap::new(language, base).expect("base map");
        map.apply_edit(&base_rope, range.clone(), inserted);
        map.refresh(&Rope::new(&edited));

        let fresh = HighlightMap::new(language, &edited).expect("fresh map");
        assert_eq!(
            map.spans, fresh.spans,
            "incremental spans differ from full re-parse (range={range:?} inserted={inserted:?})"
        );
    }

    #[test]
    fn markdown_edit_equivalence() {
        let base = "# Heading One\n\nSome **bold** and *italic* text with `code`.\n\n- item one\n- item two\n\n```rust\nfn main() {}\n```\n";
        // Insert inside a paragraph.
        let pos = base.find("**bold**").unwrap();
        assert_edit_equivalence(CodeLanguageKey::Markdown, base, pos..pos, "a");
        // Insert a new heading line at the top.
        assert_edit_equivalence(CodeLanguageKey::Markdown, base, 0..0, "## New\n\n");
        // Edit inside a fenced code block.
        let fence = base.find("fn main").unwrap();
        assert_edit_equivalence(CodeLanguageKey::Markdown, base, fence..fence + 2, "let");
        // Delete across lines.
        let start = base.find("- item one").unwrap();
        let end = base.find("- item two").unwrap();
        assert_edit_equivalence(CodeLanguageKey::Markdown, base, start..end, "");
        // Append at the end.
        assert_edit_equivalence(
            CodeLanguageKey::Markdown,
            base,
            base.len()..base.len(),
            "## Tail\n",
        );
    }

    #[test]
    fn rust_edit_equivalence() {
        let base = "fn main() {\n    let x = 1;\n    println!(\"{x}\");\n}\n";
        assert_edit_equivalence(CodeLanguageKey::Rust, base, 10..10, "x");
        assert_edit_equivalence(CodeLanguageKey::Rust, base, 0..3, "pub");
        assert_edit_equivalence(
            CodeLanguageKey::Rust,
            base,
            base.len()..base.len(),
            "\n\nfn extra() {}\n",
        );
    }

    #[test]
    fn spans_in_range_clips_to_intersection() {
        let source = "fn main() {}\n";
        let map = HighlightMap::new(CodeLanguageKey::Rust, source).expect("map");
        let all = map.spans_in_range(0..source.len());
        assert!(!all.is_empty());
        // A slice inside the keyword.
        let kw = map.spans_in_range(0..1);
        assert_eq!(kw.len(), 1);
        assert_eq!(kw[0].range, 0..1);
    }

    #[test]
    fn markdown_classes_present() {
        let source = "# Heading One\n\nSome **bold** and *italic* text with `code` and a [link](https://example.com).\n\n- item one\n- item two\n\n> quoted line\n\n```rust\nfn main() {}\n```\n";
        let result = highlight_code_block(Some("markdown"), source).expect("highlight");
        let classes: std::collections::HashSet<_> =
            result.spans.iter().map(|span| span.class).collect();
        assert!(
            classes.contains(&CodeHighlightClass::MarkupHeading(1)),
            "h1"
        );
        assert!(classes.contains(&CodeHighlightClass::MarkupBold), "bold");
        assert!(
            classes.contains(&CodeHighlightClass::MarkupItalic),
            "italic"
        );
        assert!(
            classes.contains(&CodeHighlightClass::MarkupCode),
            "inline code"
        );
        assert!(
            classes.contains(&CodeHighlightClass::MarkupLink),
            "link text"
        );
        assert!(classes.contains(&CodeHighlightClass::MarkupUri), "link uri");
        assert!(
            classes.contains(&CodeHighlightClass::MarkupList),
            "list markers"
        );
        assert!(
            classes.contains(&CodeHighlightClass::MarkupQuote),
            "quote marker"
        );
        assert!(
            classes.contains(&CodeHighlightClass::Keyword),
            "rust keyword"
        );
    }

    #[test]
    fn multibyte_markdown_never_panics() {
        // tree-sitter-md can report node edges inside multi-byte characters;
        // the engine must clamp them instead of panicking on string slicing.
        let samples = [
            "# 标题二\n\n正文内容。\n",
            "## 第二章\n\n一段中文正文。\n\n```rust\nfn main() {}\n```\n",
            "### 三\n\n文字 `code` 和链接 [x](y)。\n",
            "一二三四五六七八九十\n",
            "# 标题\n\n段落文本内容\n\n```python\nprint('你好')\n```\n\n结尾。\n",
        ];
        for text in samples {
            // The point is crash-freedom, not highlight density: plain-text
            // paragraphs legitimately produce no markup spans.
            let map = HighlightMap::new(CodeLanguageKey::Markdown, text).expect("markdown map");
            let _ = map.spans().len();
        }
    }

    /// Compares per-keystroke `apply_edit + refresh` (incremental) against
    /// rebuilding the map from scratch (full parse + full query). Run with
    /// `cargo test -p syntax_highlighter --release -- --ignored`.
    #[test]
    #[ignore = "perf benchmark"]
    fn incremental_vs_full_parse_benchmark() {
        use std::time::Instant;

        let mut base = String::new();
        for i in 0..300 {
            base.push_str(&format!(
                "fn func_{i}(value: u32) -> u32 {{\n    let doubled = value * 2;\n    doubled + {i}\n}}\n\n"
            ));
        }

        // 200 simulated keystrokes at the middle of the document.
        let edits: Vec<(usize, &'static str)> = (0..200)
            .map(|i| (base.len() / 2 + i, if i % 7 == 0 { "x\n" } else { "x" }))
            .collect();

        // Incremental path: one map, edited 200 times.
        let mut map = HighlightMap::new(CodeLanguageKey::Rust, &base).expect("map");
        let mut text = base.clone();
        let start = Instant::now();
        for (pos, inserted) in &edits {
            let old_rope = Rope::new(&text);
            text.insert_str(*pos, inserted);
            map.apply_edit(&old_rope, *pos..*pos, inserted);
            map.refresh(&Rope::new(&text));
        }
        let incremental = start.elapsed();

        // Full path: rebuild the map from scratch after every edit.
        let mut text = base.clone();
        let start = Instant::now();
        for (pos, inserted) in &edits {
            text.insert_str(*pos, inserted);
            std::hint::black_box(HighlightMap::new(CodeLanguageKey::Rust, &text));
        }
        let full_rebuild = start.elapsed();

        eprintln!("incremental 200 edits: {incremental:?}; full rebuild 200x: {full_rebuild:?}");
    }

    /// Measures the per-edit cost of [`HighlightMap::apply_edit`] (tree
    /// edits plus injection-range maintenance) on a markdown document, at
    /// several sizes. Run with `cargo test -p syntax_highlighter
    /// --release -- --ignored spans_apply_edit_benchmark`.
    #[test]
    #[ignore = "perf benchmark"]
    fn spans_apply_edit_benchmark() {
        use std::time::Instant;

        for size_kb in [64usize, 256, 1024] {
            let mut text = String::new();
            while text.len() < size_kb * 1024 {
                text.push_str(
                    "# Heading\n\nSome **bold** and `code` text.\n\n- item one\n- item two\n\n",
                );
            }
            text.truncate(size_kb * 1024);
            let map = HighlightMap::new(CodeLanguageKey::Markdown, &text).expect("map");
            let mut map = map;
            let mut rope = Rope::new(&text);
            let edits = 120;
            // Warm-up: the first few edits pay for CPU frequency ramp and
            // allocator settling; only the steady state is reported.
            for _ in 0..10 {
                let at = rope.len() / 2;
                let old_rope = rope.clone();
                map.apply_edit(&old_rope, at..at, "x");
                rope = rope.edit(at..at, "x");
            }
            let start = Instant::now();
            for i in 0..edits {
                let at = rope.len() / 2;
                let old_rope = rope.clone();
                let inserted = if i % 2 == 0 { "x" } else { "y\n" };
                map.apply_edit(&old_rope, at..at, inserted);
                rope = rope.edit(at..at, inserted);
            }
            let elapsed = start.elapsed();
            eprintln!(
                "apply_edit[{size_kb}KB]: {edits} edits = {elapsed:?} ({}us/edit, {} spans, {} layers, {} layer ranges)",
                elapsed.as_micros() / edits as u128,
                map.spans().len(),
                map.layers.len(),
                map.layers.iter().map(|l| l.chunks.len()).sum::<usize>()
            );
        }
    }
}
