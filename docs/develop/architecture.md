# Architecture

Splitype is a block-based Markdown editor built on GPUI. This document
describes the crate architecture after the stage-3 refactor: two fully
independent editor cores, a thin editor contract, and a composition-root
app crate.

## Dependency graph (strictly acyclic, CI-enforced)

```
primitives
   ├─ splitter ──→ primitives + gpui
   ├─ config ──→ primitives
   ├─ theme ──→ primitives + config
   ├─ i18n ──→ theme + config + primitives
   ├─ net ──→ primitives
   ├─ ui ──→ primitives + theme + i18n + config + splitter
   ├─ explorer_fs ──→ primitives
   ├─ workspace ──→ splitter + theme + primitives
   ├─ editor ──→ primitives + splitter + workspace + gpui      (pure contract)
   ├─ editor_wysiwyg ──→ editor + theme + i18n + ui + config + workspace + gpui
   │                    (self-hosts markdown/tree/latex/mermaid/export/highlight)
   ├─ editor_source_code ──→ editor + theme + gpui             (self-hosts tree/highlight)
   ├─ editor_preview ──→ editor + editor_wysiwyg + theme + gpui
   ├─ editor_outline ──→ editor + theme + i18n + ui + gpui
   ├─ editor_search ──→ editor + editor_wysiwyg + theme + i18n + ui + config + gpui
   ├─ explorer ──→ explorer_fs + theme + i18n + ui + workspace + gpui
   ├─ settings ──→ config + theme + i18n + ui + splitter + workspace + gpui
   └─ app ──→ everything above (composition root: Shell, Editor entity,
              bootstrap, CLI, platform glue, export flow)
```

`cargo machete` and `cargo tree` audits in CI keep this graph honest.

## The dual-core world (P2)

The two editing modes are **fully independent crates with zero shared
logic** (D14-B). They share nothing: no markdown parser, no tree
implementation, no syntax highlighter.

- **`editor_wysiwyg`** — the complete Markdown editing world: its own
  markdown parser (`markdown/`, WYSIWYG 1:1-line plus a CommonMark
  parse), its own B-tree (`tree/`), the block document model
  (`document/`), projection, input, rendering, LaTeX/Mermaid SVG
  services, HTML/PDF export, and the code-block highlighter
  (`highlight/` + `code_language/`).
- **`editor_source_code`** — the raw source editing world: its own text
  buffer state, its own highlight engine (`highlight/`, a self-hosted
  copy that evolves independently), and the line text-run builder.

Each core implements `editor::Pane`; nothing in either crate references
the other or the `Editor` entity.

## The editor contract crate (P3)

`crates/editor` is a pure contract layer:

- `EditorPaneKind`, `PaneId`, `TabKind`, `OpenFileMode` — the pane-kind
  vocabulary.
- `Pane` — the plugin trait every mode implements (`kind`,
  `document_source`, `set_search_matches`, `outline_items`, downcast
  accessors).
- `EditorDocument` — the minimal document view modes may read.
- `EditorHost` — the dependency-inversion seam to the window shell
  (the editor family never names a shell type).
- `OutlineNode` and the `outline_headings_from_markdown` service.
- `PaneFactoryRegistry` — the single pane creation point; the app
  composition root registers one factory per mode kind.

Modes depend on `editor`; `editor` depends on nothing above it.

## The presentation contract (P3.2-2)

`editor_wysiwyg::presentation` is the **only** face the preview pane may
consume from the editing world: list markers, quote/callout colors,
centered-column math, the HTML style tree, table measurement, graphic
placeholders/error cards, the LaTeX/Mermaid preview box, and the
footnote registry data contract. Editing internals (document/block
mutation, input, history, selection) are never exposed. Module
visibility plus machete/tree audits enforce the boundary.

## The app composition root (P4)

`crates/app` owns everything that names the concrete types:

- The `Editor` entity and its coordination layer (session, tabs, input
  routing, commands, chrome, document pane, search/outline overlays).
  The aggregate root references every mode type, so it lives with the
  composition root — not in `crates/editor` (see `decisions.md`, ADR-01).
- The window shell (`Shell`), window chrome, menus, dialogs.
- Bootstrap, CLI, platform glue, assets.
- The export flow (D16): export commands live here, take the active
  editor's `Pane::document_source()`, render through
  `editor_wysiwyg::export`, and write via `explorer_fs`.
- Pane factory registration.

## Panel crates

`explorer` (file tree sidebar; state is an app-wide gpui `Global`,
shell interaction goes through `workspace` actions) and `settings`
(same pattern) are standalone consumers of the infrastructure crates.

## Testing (stage 5)

Per-crate unit tests live as sibling `*_tests.rs` files; every crate
that needs GPUI declares `gpui` with the `test-support` feature in
`[dev-dependencies]` (R10). Cross-crate behavior (e.g. the Pane
contract) is exercised through each crate's test-support context and
`editor::test::TestHost` (an `EditorHost` test double). There are no
integration `tests/` directories (D4).
