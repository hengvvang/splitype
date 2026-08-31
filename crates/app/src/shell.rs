//! The window shell — the OS window's root entity.
//!
//! Owns the mapping from layout panels to content entities ([`PanelView`]),
//! the window-level chrome state ([`MenuBarState`]), and orchestrates
//! window layout, dialogs, and lifecycle guard.

pub mod close_guard;
pub mod host_bridge;
pub mod lifecycle;
pub mod view;

use std::collections::HashMap;
use gpui::*;

pub(crate) use self::host_bridge::{ShellEditorHost, ShellPanelHost};
use crate::chrome::MenuBarState;
use crate::dialogs::InfoDialogKind;
use crate::layout::WindowPanels;
use editor::{DocumentTab, Editor};
use window::{PanelId, PanelView};

/// Scope of an unsaved-changes confirmation dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UnsavedDialogScope {
    /// Triggered by titlebar window close, App Menu "Close Window" / "Quit", or Cmd/Ctrl+Shift+W.
    /// Targets ALL editor panels in the window and ALL open tabs.
    Window,
    /// Triggered by Panel topbar close button, or "Close Panel" command.
    /// Targets ONLY the specified Panel and its tabs.
    Panel(PanelId),
    /// Triggered by a specific tab's 'x' close button, or Cmd/Ctrl+W.
    /// Targets ONLY the single specified tab.
    Tab { panel_id: PanelId, index: usize },
}

impl UnsavedDialogScope {
    pub(crate) fn panel_id(&self) -> Option<PanelId> {
        match self {
            Self::Window => None,
            Self::Panel(panel_id) => Some(*panel_id),
            Self::Tab { panel_id, .. } => Some(*panel_id),
        }
    }
}

/// State for the window-level unsaved-changes confirmation dialog.
#[derive(Clone, Debug)]
pub(crate) struct UnsavedDialogState {
    pub(crate) scope: UnsavedDialogScope,
    pub(crate) document_name: String,
    pub(crate) restore_focus: Option<EntityId>,
}

/// Durable state of a panel that suspended itself during a kind switch.
/// Restored through the owning descriptor when its kind returns.
pub struct RetainedPanel {
    pub kind: window::PanelKind,
    pub state: Box<dyn std::any::Any>,
}

/// The OS window's root entity: content panel views + window lifecycle.
pub struct Shell {
    /// Polymorphic panel views implementing [`PanelView`].
    pub(crate) panel_views: HashMap<PanelId, Box<dyn PanelView>>,
    /// Suspended panel states, keyed by panel id. A panel whose kind changed
    /// away can park its documents here so they survive the switch and are
    /// restored when the kind returns.
    pub(crate) retained_panel_states: HashMap<PanelId, RetainedPanel>,
    /// Open/hover state for the in-window titlebar menu bar.
    pub(crate) menu_bar: MenuBarState,
    /// Window panel state: the outer area layout tree plus the explorer /
    /// settings sidebar states.
    pub(crate) panels: WindowPanels,
    /// The last rendered viewport size.
    pub(crate) last_viewport: Option<Size<Pixels>>,
    /// Informational dialog shown from the Help menu (About / update check).
    pub(crate) info_dialog: Option<InfoDialogKind>,
    /// Unsaved changes confirmation dialog state (Window, Editor Panel, or Single Tab).
    pub(crate) unsaved_dialog: Option<UnsavedDialogState>,
    /// True while an online update check is running in the background.
    pub(crate) update_check_in_progress: bool,
    /// Whether the window-close guard callback is installed on the window.
    pub(crate) close_guard_installed: bool,
    /// Randomized emoji indices for the About dialog background grid.
    pub(crate) about_bg_emojis: Vec<usize>,
}

impl Shell {
    /// The active editor area's active tab, if any.
    pub(crate) fn active_editor_tab<'a>(&self, cx: &'a App) -> Option<&'a DocumentTab> {
        let panel = self.active_editor_panel()?;
        let editor = self.editor_for(panel)?;
        editor.read(cx).active_tab()
    }

    /// The window's primary (first) editor area content, if any.
    pub(crate) fn primary_editor(&self) -> Option<&Entity<Editor>> {
        self.panel_views
            .values()
            .find_map(|view| {
                view.as_any()
                    .downcast_ref::<editor::EditorPanelView>()
                    .map(|p| &p.editor)
            })
    }

    /// The currently active/focused editor, or falls back to primary_editor.
    pub(crate) fn active_editor(&self) -> Option<&Entity<Editor>> {
        self.panels
            .layout
            .active_leaf
            .and_then(|leaf| self.editor_for(leaf))
            .or_else(|| self.primary_editor())
    }

    /// Recomputes every editor area's pushed state.
    pub(crate) fn sync_panel_states(&mut self, cx: &mut Context<Self>) {
        let Some(_viewport) = self.last_viewport else {
            return;
        };
        let active_tab_path = self.active_editor_tab(cx).and_then(|tab| tab.file.path.clone());
        explorer::ExplorerState::set_active_file(cx, active_tab_path);
    }

    /// Closes the explorer row context menu, if open.
    pub(crate) fn close_explorer_file_menu(&mut self, cx: &mut Context<Self>) {
        let was_open = explorer::ExplorerState::update(cx, |state, _cx| state.file_menu.take().is_some());
        if was_open {
            cx.notify();
        }
    }

    /// Opens or closes the area-type dropdown of `panel_id` (topbar click).
    pub(crate) fn toggle_panel_dropdown(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_dropdown(panel_id.into().0);
        cx.notify();
    }

    /// Shows the Help-menu informational dialog (About / update check).
    pub(crate) fn show_info_dialog(&mut self, kind: InfoDialogKind, cx: &mut Context<Self>) {
        if self.is_unsaved_dialog_open(cx) {
            return;
        }
        if kind == InfoDialogKind::About {
            use std::time::SystemTime;
            let seed = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(42);
            let mut rng_state = seed as u64;
            if rng_state == 0 {
                rng_state = 0xdeadbeef;
            }
            self.about_bg_emojis = (0..80)
                .map(|_| {
                    rng_state ^= rng_state << 13;
                    rng_state ^= rng_state >> 7;
                    rng_state ^= rng_state << 17;
                    (rng_state as usize) % 18
                })
                .collect();
        }
        self.info_dialog = Some(kind);
        cx.notify();
    }

    /// Closes the Help-menu informational dialog, if open.
    pub(crate) fn hide_info_dialog(&mut self, cx: &mut Context<Self>) {
        if self.info_dialog.take().is_some() {
            cx.notify();
        }
    }

    /// True when the unsaved-changes dialog is active.
    pub(crate) fn is_unsaved_dialog_open(&self, _cx: &App) -> bool {
        self.unsaved_dialog.is_some()
    }

    /// Toggles the maximized state of `panel_id`'s tile (topbar click).
    pub(crate) fn toggle_panel_maximize(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_maximize(panel_id.into().0);
        self.sync_panel_states(cx);
        cx.notify();
    }

    /// The editor content entity of `panel_id`, if the area holds one.
    pub(crate) fn editor_for(&self, panel_id: impl Into<PanelId>) -> Option<&Entity<Editor>> {
        self.panel_views
            .get(&panel_id.into())
            .and_then(|v| v.as_any().downcast_ref::<editor::EditorPanelView>())
            .map(|p| &p.editor)
    }

    /// Returns total number of leaves in the outer window layout tree.
    pub(crate) fn layout_leaf_count(&self) -> usize {
        self.panels.layout.tree.count_leaves()
    }
}
