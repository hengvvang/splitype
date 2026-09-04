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

use editor_contracts::Rope;

use crate::highlight::{
    CodeHighlightSpan, CodeLanguageKey, class_for_highlight, language_config,
    resolve_code_language_key,
};

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
    /// Document byte ranges this layer covers, sorted.
    ranges: Vec<Range<usize>>,
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
    /// before the edit; `range` is replaced by `inserted`. Cheap: O(edit) +
    /// O(layers) + O(spans intersecting the edit).
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
            for layer_range in &mut self.layers[idx].ranges {
                if layer_range.start >= end {
                    shift_range(layer_range, delta);
                } else if layer_range.end > start {
                    if let Some(span) = &mut dropped {
                        span.start = span.start.min(layer_range.start);
                        span.end = span.end.max(layer_range.end);
                    } else {
                        dropped = Some(layer_range.clone());
                    }
                    layer_range.start = layer_range.end; // mark for removal
                }
            }
            self.layers[idx].ranges.retain(|r| r.start < r.end);
            self.layers[idx].tree.edit(&edit);
            if self.layers[idx].ranges.is_empty() {
                self.layers.remove(idx);
            } else {
                idx += 1;
            }
        }
        self.dropped_layers = match (self.dropped_layers.take(), dropped) {
            (Some(a), Some(b)) => Some(a.start.min(b.start)..a.end.max(b.end)),
            (a, b) => a.or(b),
        };

        // Spans: drop those intersecting the edit, shift those after it.
        // Until the next refresh the edited region renders unhighlighted
        // (stale-while-revalidate).
        let mut spans: Vec<CodeHighlightSpan> = Vec::with_capacity(self.spans.len());
        for span in self.spans.iter() {
            if span.range.end <= start {
                spans.push(span.clone());
            } else if span.range.start >= end {
                spans.push(CodeHighlightSpan {
                    range: shift_usize(span.range.start, delta)..shift_usize(span.range.end, delta),
                    class: span.class,
                });
            }
        }
        self.spans = Arc::from(spans);

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
                layer
                    .ranges
                    .retain(|r| r.start >= recover.end || r.end <= recover.start);
                layer.ranges.extend(new_ranges);
                layer.ranges.sort_by_key(|r| r.start);
                if !layer.ranges.is_empty() {
                    layer.tree = parse(
                        &(config.grammar)(),
                        text,
                        &layer.ranges,
                        rope,
                        Some(&layer.tree),
                    );
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
                    ranges: new_ranges,
                });
            }
        }
        self.layers.retain(|layer| !layer.ranges.is_empty());

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
                .ranges
                .iter()
                .filter(|r| r.start < recover.end && r.end > recover.start)
                .cloned()
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
/// tree-sitter's own point convention.
fn ts_point(rope: &Rope, offset: usize) -> Point {
    let offset = offset.min(rope.len());
    let (row, col) = rope.offset_to_point(offset);
    if rope.slice(offset..offset.saturating_add(1)) == "\n" {
        Point::new(row + 1, 0)
    } else if offset == rope.len() && rope.ends_with_newline() {
        Point::new(row + 1, 0)
    } else {
        Point::new(row, col)
    }
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

fn shift_range(range: &mut Range<usize>, delta: isize) {
    range.start = shift_usize(range.start, delta);
    range.end = shift_usize(range.end, delta);
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
                        language = Some(text[capture.node.byte_range()].to_string());
                    }
                    "injection.content" => content = Some(capture.node.byte_range()),
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
                    range: capture.node.byte_range(),
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
}
