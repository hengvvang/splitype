# Editor Inner Panels — Layout, Tabs, and Focus

How the editor's internal panel system works: the split tree that arranges
panels, how splits are created, how panels stay bound to the active tab, and
how the focused panel drives the status bar.

## Overview

The editor window hosts one or more **inner panels** (views). Every panel
renders the **same active tab** — the panel layout controls *structure*, the
active tab controls *data*, and the focused panel controls *targets* for
status-bar actions. The three concerns are fully decoupled:

| Concern | Owner |
| --- | --- |
| Layout structure | `WindowLayout.editor_inner_panel_layouts` (split tree) |
| Data source | The editor's active tab |
| Operation target | `WindowLayout.focused_editor_inner_panel` |

## Data Model

```rust
// One split tree per window-level area (the editor area contains panels).
editor_inner_panel_layouts: HashMap<area_id, SplitTree<EditorInnerPanelKind>>

// Panel kinds.
enum EditorInnerPanelKind { Wysiwyg, SourceCode, Preview, Outline }

// Recursive binary split tree.
enum SplitTree<T> {
    Leaf { id: usize, kind: T },
    Split { id: usize, direction: Axis, ratio: f32, first: Box<SplitTree<T>>, second: Box<SplitTree<T>> },
}

// Globally unique focused panel (not per area).
focused_editor_inner_panel: Option<(area_id, panel_id)>
```

All of this lives in editor-level state (`Editor.panels.layout`), so the
layout survives tab switches — switching tabs only changes which document
every panel renders.

## Split Operations

### Two split entry points

| Entry | Function | New panel kind |
| --- | --- | --- |
| Status-bar split buttons (H / V) | `split_editor_inner_panel` | Inherits the **focused** panel's kind, ratio `0.5` |
| Corner drag on a panel | `split_editor_inner_panel_with_ratio` | Inherits the **dragged** panel's kind, ratio from the gesture |

If the target panel's kind cannot be resolved (should not happen), the new
panel falls back to `SourceCode`.

### Adjustments

| Operation | Function |
| --- | --- |
| Drag the splitter bar | `update_editor_inner_panel_splitter_drag` — changes the ratio only |
| Close a panel | `close_editor_inner_panel` — only removes a leaf when more than one remains |
| Change a panel's kind | `change_editor_inner_panel_kind` — via the panel header dropdown |

## Tab Binding

- Every panel renders the **active tab** document:
  - `Wysiwyg` uses the primary rendered content of the active tab.
  - `SourceCode` edits a view of the same document (edits sync back).
  - `Preview` / `Outline` are derived from the same document.
- The split tree is editor-level state: it does not reset on tab switches.
- Welcome state (no tabs): every panel shows the welcome prompt; splitting
  still works so a layout can be prepared before opening a document.
- Status-bar items that depend on a document (kind button, cursor position,
  word count) are hidden without an active tab.

## Focus Design

```rust
focused_editor_inner_panel: Option<(area_id, panel_id)>
```

- **Global uniqueness**: exactly one focused panel across the whole editor.
- **Set when**: clicking a panel (`on_mouse_down`); the first panel is
  auto-focused on first render; after closing the focused panel the next
  render auto-focuses the first remaining one.
- **Consumed by** the status bar of the matching area (it filters the global
  focus by `area_id`):
  - **Kind button** — shows the focused panel's kind; click opens the
    kind-switch dropdown for that panel.
  - **Split buttons** — target the focused panel; the new panel inherits its
    kind.
  - **Close button** — closes the focused panel (only shown with multiple
    panels).

## End-to-End Flow

```
Click a panel          → focused = (area_id, panel_id)
Status bar (that area) → shows focused panel info
Split button           → new panel inherits focused kind, inserted at 0.5
Corner drag            → new panel inherits dragged kind, inserted at gesture ratio
Every panel renders    → the same active tab document
Switch tab             → layout unchanged; every panel switches to the new document
```

## Code Locations

| Concern | File |
| --- | --- |
| Layout state (split trees, focus, drag sessions) | `src/layout/state.rs` |
| Split tree operations | `src/layout/tree.rs` |
| Panel kinds | `src/layout/types.rs` |
| Inner panel rendering | `src/editor/panels/layout/mod.rs` |
| Status-bar buttons and focus display | `src/windows/editor/status_bar.rs` |
