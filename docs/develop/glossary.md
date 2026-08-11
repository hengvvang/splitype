# Editor Vocabulary (Glossary)

One term per concept. Every new identifier must follow this table; when a
concept needs a new name, extend the table instead of inventing a synonym.

Status legend: **done** = already renamed in the codebase, **planned** = the
target name for an upcoming rename, **kept** = deliberately left as-is.

## Coordinate spaces

The block text exists in three spaces. Offsets are always qualified by the
space they live in (`source_range`, `plain_range`, `display_range`).

| Space | Name | Meaning | Examples |
| --- | --- | --- | --- |
| Markdown text | **source** | The block's serialized Markdown (or the whole document's). Produced by serialization; consumed by parsing. | `SourceOffsetMap`, `source_offset_map()`, `display_range_to_source_range`, `apply_source_space_text_edit`, `source_range_to_display_range` |
| Fragment-tree text | **plain** | The stored text: no markers, styles live on fragments. | `RichText`, `plain_to_source`, `display_to_plain_range`, `selection_plain_range`, `source_range_to_plain_range` |
| Projected text | **display** | What the caret moves through on a focused block: plain text with the touched delimiters expanded. | `display_cache`, `display_text`, `plain_to_display_range`, `replace_text_in_display_range` |

Notes:

- "visible" is **not** a coordinate name. It survives only as prose in
  comments. Done: `RichText::visible_text/visible_len` → `plain_text`/
  `plain_len`; `Block::visible_len` → `display_len`;
  `InlineRenderCache::visible_text/visible_len` → `text()`/`len()`;
  `RichText::remove_visible_prefix` → `remove_plain_prefix`;
  `replace_visible_range_with_link_references` →
  `replace_plain_range_with_link_references`;
  `replace_visible_range_raw` → `replace_plain_range_verbatim`.
- `source` also names the image origin concept (`ImageResolvedSource`).
  These two senses are unrelated; do not add new "source" names for other
  concepts.
- `visible_to_normalized` inside the inline normalizer maps *input* offsets
  (markdown when parsing, plain when editing) to *tree* offsets — it is
  input-side naming, not a coordinate.

## Blocks and the document tree

| Concept | Term | Status | Notes |
| --- | --- | --- | --- |
| A node in the document | **block** | done | The parser's old "node" vocabulary is being retired (`parser.rs`). |
| Block persistent data | **BlockData** | done | Runtime field `Block.record` → `data` (planned). |
| Kind of a block | **BlockKind** | done | `BlockKind::Callout(CalloutKind)` reads naturally. |
| Flattened document sequence | **entries** | done | `BlockEntry` (was `RenderedBlock`), `flatten_entries()`, `index_for_entity_id()`, `last_descendant()`. "Visible order" survives only as prose. |
| Block content text | **text** | done | Was "title". `apply_text_edit`, `set_block_text_and_kind`, `split_text`. |
| Markdown-syntax title | **title** | kept | Link/image title attributes (`InlineLink::title`, `ImageTarget::title`), window and dialog titles. |

## Containers and groups

| Concept | Term | Status |
| --- | --- | --- |
| Callout flavor (Note/Tip/…) | **CalloutKind** | kept — "callout type" is the ecosystem term (GFM/Obsidian `[!TYPE]`). The field name `variant` is fine; they are synonyms, do not "unify" them. |
| Visual group anchor ids | `*_group_id` (`quote_group_id`, `callout_group_id`, `footnote_group_id`) | planned (currently `*_group_anchor`) |
| Selection anchor | `anchor`/`focus` | kept — standard selection vocabulary. |

## Areas and panes

| Concept | Term | Status |
| --- | --- | --- |
| Window-level layout node | **panel** (`NodeId`) | kept — `Editor.panel_id`, `WindowPanelKind`, `activate_panel`, `panel_mode()`. |
| Editor-internal split leaf | **pane** (`usize`) | kept — `focused_pane`, `split_pane`, `EditorPaneKind`, `source_pane_runtimes`. |

Never mix the two in comments or identifiers: panels belong to the Shell
layout, panes belong to an Editor session.

## Inline structure taxonomy

| Concept | Term | Notes |
| --- | --- | --- |
| Model text unit | **fragment** | `InlineFragment`, `RichText.fragments`. |
| Styled range over text | **span** | `InlineSpan` (render cache), `LinkSpan`/`FootnoteSpan` (projection, planned). |
| Rendered/syntax run | **run** | GPUI `TextRun`, backtick run. External vocabulary. |
| Role-bearing piece | **segment** | `ExpandedInlineSegment`, `PlannedInnerSegment`, `TableCellInlineImageSegment`. |

## Editing semantics

| Concept | Term | Status |
| --- | --- | --- |
| View mode (whole document) | `EditorMode::Wysiwyg/SourceCode` | kept — user-facing. |
| Block edit mode | `RenderedRich` / `Verbatim` / `CodeBlockRaw` | done — `SourceRaw` → `Verbatim`; the mode enum carries the behavior, "raw" stays for content. |
| Opaque passthrough content | **raw** | kept for content: `BlockKind::RawMarkdown`, `raw_source` (unparsed original text). |
| Marker-free editing of raw content | **verbatim** | done — `BlockEditMode::Verbatim` (was `SourceRaw`), `edits_verbatim_text` (was `uses_raw_text_editing`), `is_verbatim_mode`/`set_verbatim_mode` (was `is_/set_source_raw_mode`), `replace_plain_range_verbatim` (was `replace_visible_range_raw`). `set_source_document_mode` stays: it means SourceCode-view document mode. |
| Derived view state | — | done — "Runtime" no longer names editor state; it is reserved for background execution (`tokio::runtime`, `SyncRuntime`). `TableRuntimes` → `TableGrids`, `rebuild_table_runtimes` → `rebuild_table_grids`, `Block.table_runtime` → `table_grid`, `install_table_runtime_for_block` → `install_table_grid_for_block`, `sync_table_record_from_runtime` → `sync_table_record_from_grid`, `table_runtime.rs` → `table_grid.rs`; `SourceCodePanelRuntime` → `SourceCodePaneState`, `source_pane_runtimes` → `source_pane_states`; `image_runtime` → `image_handle`; `rebuild_image_runtimes` → `rebuild_reference_registries` (it rebuilds the image/link/footnote registries), `sync_runtime_after_block_change` → `sync_references_after_block_change`, `sync_runtime_context_for_block` → `sync_reference_context_for_block`, `set_runtime_context` → `set_reference_context`. "Runtime-only blocks" prose stays: table cells exist only in the runtime tree. |
| Document serialization | `serialize_*` | done — `Document::to_markdown` → `serialize_markdown`, `Document::to_raw_source` → `serialize_source_text`, `Editor::current_document_source` → `serialize_document_for_mode`, `BlockData::markdown_line` → `serialize_markdown_line`. Internal collectors keep their verb (`collect_root_markdown_lines`, `collect_single_block_markdown_lines`, `collect_markdown_lines`); `serialize_table_markdown_lines` already followed the prefix. |

## Architecture invariants

- **One document rebuild entry**: `Editor::rebuild_document_from_markdown`
  (mode-aware) is the only place that replaces the whole document from
  Markdown. Undo, cross-block edits, view-mode switches, drop, and source
  panes all route through it.
- **Derived views re-sync on edits, not per frame**: `DocumentTab.document_revision`
  is bumped by `mark_dirty` and the few mutation paths that bypass it;
  preview and source panes compare their `synced_revision` against it and
  skip whole-document serialization on unchanged frames.
- **Per-keystroke registry sync is incremental**: `sync_references_after_block_change`
  skips the document-wide image/link/footnote rebuild when the edited block
  is not a registry candidate (`ReferenceRegistries.candidate_blocks`).
- **Two edit routes**: in-block edits mutate the fragment tree directly;
  cross-block edits serialize, splice, and re-parse via the single rebuild
  entry. Do not add a third route.
- **Focused/unfocused render paths**: a focused block always renders through
  `BlockTextElement` (editable, projection, IME); unfocused blocks may use
  the mixed-inline-visuals div tree. Keep the geometry layer shared.

## Mechanical rename notes

- GNU sed BRE: `\(` `\)` open a *group*, not literal parens. Match literal
  parens unescaped (`s/foo()/bar()/g`).
- `\b` does not match before/after `_`; rename multi-word identifiers with
  their full underscore form, not a bare word pattern.
- `&` in a sed replacement expands to the whole match; escape it as `\&`.
- A rename is only done when `cargo test --workspace` passes with zero
  changes to test expectations.
