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

**Status:** Proposed

`PaneKind` and `PanelKind` will move from `&'static str` to owned, namespaced IDs.
The registry rejects collisions and records contribution ownership. Persisted
layout never stores references into plugin code or dynamic-library memory.

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
`SettingsPanelView` already owns its `SettingsUiState` entity; Explorer will
follow the same pattern. Application globals are reserved for true shared
services such as configuration and theme management.
