# Decision Records

Every design decision that shaped the codebase, in ADR style. The
user-approved decisions D1–D16 from the refactor plan are summarized
first; the ADRs below record rulings made while executing the plan.

## Approved decisions (plan v2.4, user rulings)

- **D1** Dual editor cores fully independent; preview → wysiwyg narrow.
- **D2** Orphan benches deleted.
- **D3** Crate split; `markdown`/`sum_tree`/`syntax`/`latex`/`mermaid`/`export`
  dissolved into `editor_wysiwyg` / `editor_source_code`.
- **D4** No integration `tests/` directories; per-crate tests only.
- **D5** `input/runtime.rs` three-way split (block factory / references / focus).
- **D6** README in scope.
- **D7** Shared editor content lives in `crates/editor`; Pane plugin architecture.
- **D8** `sum_tree` dual-vendored (wysiwyg copy + source copy), zero sharing.
- **D9** `crates/app` is the single composition root.
- **D10** `editor_outline` / `editor_search` are independent crates; app
  assembles the overlays.
- **D11** `text_layout` + `table_measure` → wysiwyg.
- **D12** `editor_preview` depends on `editor_wysiwyg`.
- **D13** `markdown` → wysiwyg.
- **D14-B** `syntax` split dual: wysiwyg self-hosts code-block highlight,
  source_code self-hosts language highlight.
- **D15** `latex` / `mermaid` / `export` → wysiwyg.
- **D16** Export flow moves to `app`.

## ADR-01 — Where the `Editor` entity lives

**Status:** accepted (execution ruling).

**Context.** Plan §3.4 places the `Editor` entity definition in
`crates/editor`, while §3.5 (the CI-enforced dependency graph) states
`editor` never depends on the mode crates. The `Editor` aggregate root
holds the session/tab model, the document, and downcast accessors for
every mode state — it necessarily names every mode type. Moving it into
`editor` would create the cycle `editor → editor_wysiwyg` and
`editor_wysiwyg → editor` (modes implement `editor::Pane`).

**Decision.** The `Editor` entity and its coordination layer live in
`crates/app` (the composition root), which already depends on every
crate. `crates/editor` remains a pure contract layer (Pane, EditorHost,
pane-kind vocabulary, factory registry). The dependency graph stays
acyclic; modes still never reference editor internals.

**Consequences.** `crates/editor` is contract-only. The app crate is
larger than §3.4 sketched. Dependency direction and CI audits are
unchanged.

## ADR-02 — `sum_tree` source copy deferred

**Status:** accepted.

**Context.** D8 requires dual-vendored trees. The WYSIWYG world uses
`SumTree<BlockData>` (`editor_wysiwyg::tree`); the source-code world's
buffer is currently a `String` with no tree consumer.

**Decision.** `sum_tree` dissolved into `editor_wysiwyg::tree`. The
source copy (`editor_source_code::buffer::tree`) is deferred until the
source buffer actually needs one — writing a second tree with no
callers would violate the zero-redundancy mandate.

**Consequences.** Zero unused code; D8's intent (no shared tree
implementation between the cores) holds — the WYSIWYG tree is
self-hosted and the source core will self-host its own when needed.

## ADR-03 — Action namespace keeps the `splitype` string

**Status:** accepted.

**Context.** The block-editing `actions!` macro namespace is a runtime
string used by keybinding config parsing; the crate was renamed
`splitype` → `app`.

**Decision.** The namespace string stays `"splitype"` (and the product
keeps the name Splitype); only the Rust crate/binary names changed.

**Consequences.** Keybinding configuration files remain valid; no
migration needed.

## ADR-04 — `highlight` self-hosting duplication is deliberate

**Status:** accepted (D14-B).

**Context.** The dissolved `syntax` crate's highlight engine is complex
(≈1000 lines: language registry, tree-sitter configs, span mapping).

**Decision.** `editor_wysiwyg::highlight` keeps the engine; the
source-code core self-hosts a copy plus its line text-run builder.
Both evolve independently; `cargo tree`/machete audits confirm zero
shared highlight code.

**Consequences.** Accepted duplication per D14-B's explicit tradeoff
(independent evolution over shared code).

## ADR-05 — Pane creation goes through the factory registry

**Status:** accepted.

**Context.** §3.6.4 requires a pane factory registry so `editor` never
names a mode type.

**Decision.** `editor::PaneFactoryRegistry` (a process-wide
`LazyLock<Mutex<…>>`, no context needed) is the single creation point;
the app bootstrap registers one factory per `EditorPaneKind`.
`PaneState` routes `new`/`ensure_kind` through it.

**Consequences.** Adding a mode kind = adding a factory registration in
one place; the editor crate stays mode-agnostic.

## ADR-06 — Mode presentation and input live in the mode crates

**Status:** accepted (execution ruling, user direction).

**Context.** After the D3 crate split the mode crates held only state;
every renderer and input handler still lived in `app/editor` as
`impl Editor` methods (the D7 migration deferred them "until the Editor
entity converges", which ADR-01 ruled out). `app/editor` carried ≈5500
lines that were purely Preview/Source/Outline/Search presentation:
`panes/preview/render/` (15 files), `panes/source_code/element.rs` +
`events.rs`, the outline HUD, and the search panel UI.

**Decision.** The mode crates now own their full presentation and input
state transitions. Coordination-layer actions (focus routing, autoscroll,
dirty marking, source sync, undo/redo, preview selection, outline
navigation, search execution) are requested through *reverse seams
defined by the consuming crate*: `editor::PaneHost` (shared, on the
contract layer) plus mode-specific hosts (`editor_outline::OutlineHost`,
`editor_search::SearchHost`). Renderers read state through snapshot
interfaces (`editor_source_code::SourceStateView`,
`editor_search::SearchStateView`) and register IME through
`SourceIme`/`SearchIme` proxies — gpui binds platform input handlers to
concrete entities, so the app implements these by re-entering the
`Editor` via its weak handle. `AutoscrollStrategy` sank from
`editor_wysiwyg::state` into the contract crate so `PaneHost` can name
it. `app/editor` keeps only coordination shells.

**Boundary (orphan rule).** `impl EntityInputHandler for Editor` must
live next to the `Editor` type (`app/editor/search/ime.rs`), so the
editor-wide IME bridge — which serves both the Source pane and the search
inputs — stays in app by Rust's orphan rule, not by choice.

**Consequences.** Mode crates never name the `Editor` type; the
`Entity<Editor>` dependency disappeared from all mode code. Each crate's
module docs state its ownership explicitly.
