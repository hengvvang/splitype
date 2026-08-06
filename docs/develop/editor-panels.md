# Editor Areas, Panels, and Mode Transitions

How the editor's internal layout works: window-level areas, the per-area
editor session (tabs + panel split tree), the welcome/editing mode model
that the panel kinds encode, and how the focused panel drives the status
bar.

## Overview

The window is divided into **areas** (`window_area_tree`), each with a kind
(`Explorer`, `Settings`, `Editor`). An `Editor` area owns an **editor
session** — its own tab list *and* its own inner panel split tree. This is
deliberate: every editor area is independent (separate tabs, separate panel
layout), and explorer file opens target the **active editor** only.

Within an editor area, every panel renders the **same active tab** of that
area — the panel layout controls *structure*, the active tab controls
*data*, and the focused panel controls *targets* for status-bar actions.

| Concern | Owner |
| --- | --- |
| Window area layout | `WindowLayout.window_area_tree` |
| Per-area tabs + panel tree | `WindowLayout.editor_sessions` |
| Panel arrangement | `EditorSession.inner_panel_tree` (split tree) |
| Data source | The area's active tab (`EditorSession.tab_list`) |
| Explorer file target | `WindowLayout.active_editor_area` (foreground only) |
| Status-bar action target | `WindowLayout.focused_editor_inner_panel` |

## Data Model

```rust
// One editor session per window area. The session aggregates the two
// things an editor area owns so they can never drift apart.
struct EditorSession {
    tab_list: EditorTabList,                              // the area's tabs
    inner_panel_tree: SplitTree<EditorInnerPanelKind>,    // the area's panels
}
editor_sessions: HashMap<AreaId, EditorSession>

// Panel kinds: the outer variant is the mode, the inner variant is the
// panel type within that mode. The tree always tells the truth.
enum EditorInnerPanelKind {
    Welcome(WelcomePanelKind),   // welcome mode (no tabs)
    Editing(EditingPanelKind),   // editing mode (has tabs)
}
enum WelcomePanelKind { Welcome(Option<EditingPanelKind>) }
enum EditingPanelKind { SourceCode, Wysiwyg, Preview, Outline }

// Recursive binary split tree.
enum SplitTree<T> {
    Leaf { id: usize, kind: T },
    Split { id: usize, direction: Axis, ratio: f32, first: Box<SplitTree<T>>, second: Box<SplitTree<T>> },
}

// Globally unique focused panel (not per area).
focused_editor_inner_panel: Option<InnerPanelLocation>   // { area_id, panel_id }
```

The layout lives in `WindowLayout`, so it survives tab switches — switching
tabs only changes which document every panel of that area renders.

## Panel Kinds and Mode Transitions

### The tree is the truth

An editor area is in one of two modes, derived from whether its session has
tabs (`editor_area_mode`):

| Mode | Tree invariant |
| --- | --- |
| `Welcome` (no tabs) | every panel is `Welcome(WelcomePanelKind)` |
| `Editing` (has tabs) | every panel is `Editing(EditingPanelKind)` |

Rendering matches on the panel kind directly — there is no separate mode
branch: `Welcome(_)` renders the welcome prompt, `Editing(k)` renders the
view for `k`.

### Transitions

The tree is migrated as a whole when the mode flips; the split structure is
always preserved:

| Transition | When | Effect |
| --- | --- | --- |
| `enter_editing(area)` | first tab is pushed (`open_file_in_area`, `new_untitled_tab`, `from_markdown`) | `Welcome(None)` → `Editing(SourceCode)`; `Welcome(Some(k))` → `Editing(k)` |
| `exit_editing(area)` | last tab is closed (`close_tab`) | `Editing(k)` → `Welcome(Some(k))` |

Both are idempotent. Because a welcome panel **remembers** the editing kind
it had before, closing the last tab and re-entering editing restores the
previous panel layout:

```
Editing(Preview) ──close last tab──▶ Welcome(Some(Preview)) ──open file──▶ Editing(Preview)
Editing(Wysiwyg) ─────────────────▶ Welcome(Some(Wysiwyg)) ─────────────▶ Editing(Wysiwyg)
(fresh area)     ─────────────────▶ Welcome(None) ───────────────────────▶ Editing(SourceCode)
```

The implementation collects the leaf ids and rewrites each leaf's kind via
`set_leaf_kind` (a generic recursive rewrite with an `impl FnMut` closure
triggers a pathological rustc 1.97 codegen slowdown, so it is deliberately
avoided).

## Split Operations

### Two split entry points

| Entry | Function | New panel kind |
| --- | --- | --- |
| Status-bar split buttons (H / V) | `split_editor_inner_panel` | Inherits the **focused** panel's kind, ratio `0.5` |
| Corner drag on a panel | `split_editor_inner_panel_with_ratio` | Inherits the **dragged** panel's kind, ratio from the gesture |

Inheritance is whole-kind, so the mode stays consistent automatically:
splitting a welcome panel produces another welcome panel (with the same
remembered kind), splitting an editing panel produces the same editing
kind. If the target cannot be resolved (should not happen), the new panel
falls back to `Welcome(None)`.

Splitting works in the welcome state too, so a layout can be prepared
before any document is opened.

### Adjustments

| Operation | Function |
| --- | --- |
| Drag the splitter bar | `update_editor_inner_panel_splitter_drag` — changes the ratio only |
| Close a panel | `close_editor_inner_panel` — only removes a leaf when more than one remains |
| Change a panel's kind | `change_editor_inner_panel_kind` — via the panel header dropdown; takes an `EditingPanelKind`, so the type system prevents switching a panel into the welcome mode |

## Areas, Sessions, and the Active Editor

- An editor area keeps its session when it is switched to another kind
  **only if the session still holds tabs** (background editing): switching
  back to `Editor` restores the tabs and the panel layout. Empty sessions
  are dropped (`change_window_area_kind`).
- The **active editor** is the last focused *foreground* editor
  (`active_editor_area` + `editor_activation_history`). `is_foreground_editor`
  is the only consumer of the foreground/background distinction; explorer
  file opens target the active editor and are ignored when no foreground
  editor exists — they never land in a background (retained) session.
- `EditorAreaMode` (`Welcome`/`Editing`) remains the area-level query for
  renderers and the status bar; the panel kinds encode the same fact
  per-panel.

## Focus Design

```rust
focused_editor_inner_panel: Option<InnerPanelLocation>   // { area_id, panel_id }
```

- **Global uniqueness**: exactly one focused panel across the whole window.
- **Set when**: clicking a panel (`on_mouse_down`); the first panel is
  auto-focused on first render; after closing the focused panel the next
  render auto-focuses the first remaining one. Interacting with an editor
  area also activates it (active-editor rule above).
- **Consumed by** the status bar of the matching area (it filters the global
  focus by `area_id`):
  - **Mode pill** — always visible: shows `Welcome` (disabled) in the
    welcome mode, or the focused panel's kind (click opens the kind-switch
    dropdown) in the editing mode.
  - **Split buttons** — target the focused panel; the new panel inherits its
    kind.
  - **Close button** — closes the focused panel (only shown with multiple
    panels).

## End-to-End Flow

```
Open a file / double-click welcome  → first tab pushed → enter_editing:
                                      Welcome(None) → Editing(SourceCode)
Click a panel                        → focused = (area_id, panel_id)
Status bar (that area)               → mode pill shows focused panel kind
Split button                         → new panel inherits focused kind, ratio 0.5
Corner drag                          → new panel inherits dragged kind, gesture ratio
Every panel renders                  → the area's active tab document
Switch tab                           → panel tree unchanged
Close the last tab                   → exit_editing: panels become
                                      Welcome(Some(kind)), layout preserved
Switch the area to Explorer          → session retained while tabs remain
                                      (background editing); empty sessions dropped
Explorer opens a file                → targets the active (foreground) editor;
                                      ignored when none exists
```

## Code Locations

| Concern | File |
| --- | --- |
| Layout state (areas, sessions, focus, drag sessions) | `src/layout/state.rs` |
| Split tree operations | `src/layout/tree.rs` |
| Panel kinds and mode types | `src/layout/types.rs` |
| Inner panel rendering | `src/editor/panels/layout/mod.rs` |
| Status-bar buttons and focus display | `src/windows/editor/status_bar.rs` |
