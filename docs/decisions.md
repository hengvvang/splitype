# Architecture decisions

## ADR-001: Treat built-ins as plugins

**Status:** Accepted

Editor, Explorer, Settings, Source, WYSIWYG, and Preview are implementation
crates selected only by the `app` composition root. Kernels depend on contracts,
not these concrete crates. Product defaults may name built-ins in the composition
root; lifecycle code may not branch on their types.

## ADR-002: Keep plugin callbacks outside registry locks

**Status:** Accepted

Registries clone a descriptor while locked and release the lock before invoking
factory or plugin code. Duplicate kinds are errors rather than implicit overrides.
This permits reentrant discovery and prevents plugin-controlled deadlocks.

## ADR-003: Share immutable document snapshots

**Status:** Accepted

The editor owns authoritative mutable document state. Every pane consumes a
`DocumentSnapshot` with stable identity, revision, text, path, and base directory.
Panes do not infer document resource roots from globals.

## ADR-004: Do not expose Rust/GPUI as a stable runtime ABI

**Status:** Accepted

Current traits are an in-process source-plugin API tied to the exact Rust and
GPUI build. Runtime third-party packages will use a versioned WASM, subprocess,
or C-compatible protocol. Rust trait objects, `Any`, GPUI entities, and Rust
containers do not cross that boundary.

## ADR-005: Prefer capabilities over kind checks and no-op methods

**Status:** Proposed

Pane and panel behavior will be selected through typed capabilities and
structured outcomes. Kernels will not compare built-in kind strings or downcast
to built-in views. Unsupported mutation cannot be interpreted as success or mark
a document dirty.

## ADR-006: Separate instance state from application services

**Status:** Proposed

Selection, focus, menus, drag sessions, and other transient state belong to each
pane/panel instance. Application globals are reserved for true shared services.
Explorer and Settings state will be migrated accordingly.

## ADR-007: Use owned, namespaced plugin identifiers

**Status:** Accepted

`PaneKind` and `PanelKind` are owned `Arc<str>` identifiers. Registries reject
collisions and record contribution ownership, and persisted layout no longer
stores references into plugin code or dynamic-library memory. The split tree
works on `Clone + PartialEq` payloads instead of requiring `Copy`.

## ADR-008: Suspend panels through their own descriptors

**Status:** Accepted

Panels own their durable state. Kind switches suspend a panel via
`PanelView::suspend_state` into `Shell::retained_panel_states` and restore it
through `PanelDescriptor::restore_panel`; dirty-state queries and discards of
parked state go through the descriptor as well. The shell treats parked state
as opaque and no longer keeps an editor-specific session map.

## ADR-009: Pane edits commit atomically

**Status:** Accepted

Host-driven pane commands return the replacement document text and the editor
applies exactly one commit: replace authoritative text, bump revision, mark
`dirty`, invalidate caches, broadcast the snapshot. Spontaneous pane edits
commit through `PaneHost::sync_source_text`. Panes no longer follow a text
commit with a second `mark_dirty`, and unsupported commands return `None`
instead of faking a dirty document.

## ADR-010: Panel instance state belongs to the panel

**Status:** Accepted

Panel plugins own transient UI state in entities created by their descriptors.
Both `SettingsPanelView` and `ExplorerPanelView` own their state entities;
explorer splits, clones, and multi-window instances are now independent.
Application globals are reserved for true shared services such as
configuration and theme management.

## ADR-011: Panes are UI-thread objects with declared capabilities

**Status:** Accepted

`PaneView` no longer requires `Send + Sync`; pane instances live on the GPUI
thread and use `RefCell` or plain fields for transient UI state. Optional
behaviors are declared through `PaneCapabilities` and hosts gate search,
replacement, editing, outline, and navigation on the declaration, so a
read-only pane can never fake an edit or dirty document.

## ADR-012: Panels route documents through a capability and trait, not kinds

**Status:** Accepted

The window shell treats the editor as an ordinary third-party panel. Panel
plugins declare `PanelCapabilities` (`documents`, `sidebar`); a panel that
declares `documents` also implements the `DocumentPanel` trait, which carries
every document lifecycle operation (open, tab save/close/discard, dirty
dialogs, drop replacement, focus, save/export). The shell routes all of these
through `dyn DocumentPanel` via opt-in downcast hooks on `PanelView` and never
downcasts to a concrete editor type or compares kind strings. Menu actions
dispatch to a panel by `PanelId` and the default window layout is derived from
registry capabilities, so any plugin can take over the editor or sidebar role.

## ADR-013: Contracts own the domain vocabulary; the shell depends on contracts

**Status:** Accepted

`core_contracts` no longer depends on the `window` shell crate. Panel and pane
contracts, kind and id types, and the document vocabulary all live in
`core_contracts`; `window` hosts the registry implementation, and every
consumer imports contract types from `core_contracts` directly — no re-export
shims. Built-in kinds are namespaced (`splitype.pane.*`, `splitype.panel.*`),
constructed via `from_static` without allocating, and icon asset paths are
decoupled from kind strings: topbar renderers take a plugin-owned
`icon_prefix`.

## ADR-014: Sidebar panels are a role, and overlays are panel-owned

**Status:** Accepted

The explorer is treated as a third-party sidebar plugin. Panels declaring
`sidebar` implement the `SidebarPanel` trait (`set_active_document_path`,
`on_document_path_changed`, `toggle_drawer`, `close_active_folder`); the shell
pushes document context and sidebar commands through the trait and no longer
imports explorer types. Window-level overlays (context menus, popovers) are
rendered by the owning panel through `PanelView::render_overlay` and dismissed
through `PanelView::dismiss_overlays`, so plugin UI is never rendered by the
shell. Plugin-emitted shell notifications (path renames) use shell-owned
`window::actions` vocabulary, keeping the dependency direction
plugin -> shell -> contracts.

## ADR-015: Window state persists through a versioned opt-in snapshot

**Status:** Accepted

Window persistence is a shell service with a versioned schema: the shell
captures the layout topology (a serde projection of the split tree; transient
drag/dropdown sessions are skipped) plus per-panel state into
`window_state.json`, gated by the `restore_window_state` setting, and
snapshots at every window-close decision point. Panels opt into state
serialization through `PanelDescriptor::serialize_state`/`deserialize_state`;
non-opting panels are recreated fresh on restore. The editor persists its full
`EditorSession` (tabs, text, dirty flags, pane layout kinds) while per-pane
runtime entities are rebuilt from the restored text. The explorer persists its
drawer visibility and open folder paths (worktrees are re-scanned from disk on
restore), and the settings panel persists its active tab. Loaders reject
snapshots whose schema version they do not understand.

## ADR-016: Plugins are declared by manifests, discovered by the composition root

**Status:** Accepted

Every plugin — built-in or third-party — is described by a versioned TOML
manifest (`PluginManifest`) with a reverse-domain `PluginId`, an entry-point
declaration, declared pane/panel kinds, and resource roots. Manifests are
recorded in a global `PluginRegistry`; the composition root is the only place
allowed to map an in-process registration key to concrete descriptor
factories, and registration validates every registered kind against the
manifest's declarations. Plugin resources are addressed through
`plugin://<id>/<path>`, resolved via the owning manifest's `icon_root` into
pluggable asset catalogs — kind strings never encode resource locations.

## ADR-017: Menus and keybindings assemble from command contributions

**Status:** Accepted

Commands are plugin contributions: manifests declare `[[commands]]` entries
(plugin-local id, menu skeleton location, default shortcuts, keybinding
context) recorded in a global `CommandRegistry`. The menu bar assembles its
static items from the registry through a fixed menu skeleton, and the
composition root installs keybindings by resolving every contribution's
shortcuts through its command binding table (the only place allowed to map
a command id to its localized label and concrete action), applying user
overrides with context-scoped conflict resolution. Dynamic menu sections
(recent files, themes, languages, CLI tool) are composition-root providers.
GPUI's typed `cx.on_action` API still requires one dispatch handler per
action type, so binding table and handlers must stay in sync.

## ADR-018: Missing plugins degrade to a named placeholder

**Status:** Accepted

A layout leaf whose kind has no registered descriptor is rendered by a
shell-owned `MissingPanelView` placeholder instead of a blank tile: the
layout stays intact and the placeholder names the owning plugin through the
plugin registry. User-installed manifests under the config `plugins/`
directory are discovered, validated, and recorded as metadata so missing
kind strings can still be named — even though code transports for user plugins do
not exist yet.

## ADR-019: One canonical path per item — no compatibility shims

**Status:** Accepted

The codebase keeps exactly one design and one path per item. Cross-crate
re-export facades are removed (consumers import contract types from
`core_contracts` only), plugin crate roots expose their descriptor as the
sole root-level entry point while internals stay at canonical module paths,
and `pane_wysiwyg` addresses `markdown_parser`/`syntax_highlighter` directly
instead of through `pub use` facades. `markdown_parser` owns its model under
`parse::*` (the `parse::parser` nesting is gone) and carries no GPUI identity.
The editor keeps action dispatch handlers in `editor::actions` while state
mutations live in the session and navigation modules; `explorer::state` is a
single module rather than `state::state`. Backward-compatibility aliases are
deleted on the spot, never kept.
