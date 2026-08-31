//! The window shell — the OS window's root entity.
//!
//! Owns the mapping from layout panels to content entities ([`PanelView`]),
//! the window-level chrome state ([`MenuBarState`]), and orchestrates
//! window layout, dialogs, and lifecycle guard.

pub mod close_guard;
pub mod host_bridge;
pub mod lifecycle;
pub mod view;

use gpui::*;
use std::collections::HashMap;

pub(crate) use self::host_bridge::{ShellDocumentHost, ShellPanelHost};
use crate::chrome::MenuBarState;
use crate::dialogs::InfoDialogKind;
use crate::layout::WindowPanels;
use core_contracts::{DocumentPanel, PanelId, PanelKind};
use splitter::tree::NodeId;
use std::path::PathBuf;
use window::PanelView;

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
    /// Path of the active document panel's active tab, if any.
    pub(crate) fn active_document_tab_path(&self, cx: &App) -> Option<PathBuf> {
        let panel = self.active_document_panel_id()?;
        self.document_panel_for(panel)?.active_tab_path(cx)
    }

    /// The first document panel in this window, if any.
    pub(crate) fn primary_document_panel_id(&self) -> Option<PanelId> {
        self.panel_views
            .iter()
            .find(|(_, view)| view.capabilities().documents)
            .map(|(id, _)| *id)
    }

    /// The active document panel, or the first document panel as fallback.
    pub(crate) fn active_document_panel_id(&self) -> Option<PanelId> {
        self.panels
            .layout
            .active_leaf
            .filter(|leaf| self.leaf_is_document_panel(*leaf))
            .map(PanelId)
            .or_else(|| self.primary_document_panel_id())
    }

    /// The active document panel, or the first document panel as fallback.
    pub(crate) fn active_document_panel_mut(&mut self) -> Option<&mut dyn DocumentPanel> {
        let panel_id = self.active_document_panel_id()?;
        self.document_panel_mut_for(panel_id)
    }

    /// Whether `leaf` currently holds a document-routing panel.
    ///
    /// Consults the live view first and falls back to the registered
    /// descriptor so unmaterialized leaves still answer correctly.
    pub(crate) fn leaf_is_document_panel(&self, leaf: NodeId) -> bool {
        if let Some(view) = self.panel_views.get(&PanelId(leaf)) {
            return view.capabilities().documents;
        }
        let Some(kind) = self.panels.layout.tree.find_leaf_kind(leaf) else {
            return false;
        };
        Self::kind_is_document_panel(&kind)
    }

    /// Whether panels of `kind` route documents, per their descriptor.
    pub(crate) fn kind_is_document_panel(kind: &PanelKind) -> bool {
        window::PanelRegistry::registered(kind.clone())
            .ok()
            .flatten()
            .is_some_and(|descriptor| descriptor.capabilities().documents)
    }

    /// The document-routing view of `panel_id`, if it has one.
    pub(crate) fn document_panel_for(
        &self,
        panel_id: impl Into<PanelId>,
    ) -> Option<&dyn DocumentPanel> {
        self.panel_views
            .get(&panel_id.into())
            .and_then(|view| view.as_document_panel())
    }

    /// The mutable document-routing view of `panel_id`, if it has one.
    pub(crate) fn document_panel_mut_for(
        &mut self,
        panel_id: impl Into<PanelId>,
    ) -> Option<&mut dyn DocumentPanel> {
        self.panel_views
            .get_mut(&panel_id.into())
            .and_then(|view| view.as_document_panel_mut())
    }

    /// The document panel currently requesting the unsaved-changes dialog.
    pub(crate) fn document_panel_with_unsaved_dialog_mut(
        &mut self,
        cx: &App,
    ) -> Option<&mut dyn DocumentPanel> {
        self.panel_views.values_mut().find_map(|view| {
            view.as_document_panel_mut()
                .filter(|panel| panel.has_unsaved_dialog(cx))
        })
    }

    /// The document panel currently requesting the drop-replace dialog.
    pub(crate) fn document_panel_with_drop_replace_dialog_mut(
        &mut self,
        cx: &App,
    ) -> Option<&mut dyn DocumentPanel> {
        self.panel_views.values_mut().find_map(|view| {
            view.as_document_panel_mut()
                .filter(|panel| panel.has_drop_replace_dialog(cx))
        })
    }

    /// Seeds the active document panel with the window's initial document.
    pub(crate) fn load_initial_document(
        &mut self,
        text: String,
        path: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = self.active_document_panel_mut() {
            panel.load_initial_document(text, path, cx);
        }
    }

    /// Captures this window's layout and per-panel plugin state and persists
    /// it as the launch-restore snapshot, when the setting is enabled.
    pub(crate) fn snapshot_window_state(&self, cx: &mut Context<Self>) {
        if !config::settings::SettingsStore::get(cx)
            .startup
            .restore_window_state
        {
            return;
        }
        let mut panels = Vec::new();
        for (panel_id, view) in &self.panel_views {
            let Some(state) = view.clone_state(cx) else {
                continue;
            };
            let Ok(Some(descriptor)) = window::PanelRegistry::registered(view.kind()) else {
                continue;
            };
            let Some(json) = descriptor.serialize_state(state.as_ref()) else {
                continue;
            };
            panels.push(window::PersistedPanel {
                id: *panel_id,
                kind: view.kind(),
                state: json,
            });
        }
        let state = window::PersistedWindowState {
            version: window::WINDOW_STATE_VERSION,
            tree: self.panels.layout.tree.clone(),
            next_node_id: self.panels.layout.next_node_id,
            active_leaf: self.panels.layout.active_leaf,
            activation_history: self.panels.layout.activation_history.clone(),
            panels,
        };
        if let Err(err) = crate::window_state::save_window_state(&state) {
            tracing::warn!(error = %err, "failed to persist window state");
        }
    }

    /// Recomputes every sidebar panel's pushed document context.
    pub(crate) fn sync_panel_states(&mut self, cx: &mut Context<Self>) {
        let Some(_viewport) = self.last_viewport else {
            return;
        };
        let active_tab_path = self.active_document_tab_path(cx);
        for view in self.panel_views.values_mut() {
            if let Some(panel) = view.as_sidebar_panel_mut() {
                panel.set_active_document_path(active_tab_path.clone(), cx);
            }
        }
    }

    /// Notifies every sidebar panel that a document's backing path changed.
    pub(crate) fn notify_sidebar_document_path_changed(&mut self, cx: &mut Context<Self>) {
        for view in self.panel_views.values_mut() {
            if let Some(panel) = view.as_sidebar_panel_mut() {
                panel.on_document_path_changed(cx);
            }
        }
    }

    /// Toggles the drawer of every sidebar panel in this window.
    pub(crate) fn toggle_sidebar_drawers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for view in self.panel_views.values_mut() {
            if let Some(panel) = view.as_sidebar_panel_mut() {
                panel.toggle_drawer(window, cx);
            }
        }
    }

    /// Closes the open folder scope of every sidebar panel in this window.
    pub(crate) fn close_sidebar_folders(&mut self, cx: &mut Context<Self>) {
        for view in self.panel_views.values_mut() {
            if let Some(panel) = view.as_sidebar_panel_mut() {
                panel.close_active_folder(cx);
            }
        }
    }

    /// Dismisses transient overlays (context menus, popovers) of every panel.
    pub(crate) fn dismiss_panel_overlays(&mut self, cx: &mut Context<Self>) {
        let mut dismissed = false;
        for view in self.panel_views.values_mut() {
            dismissed |= view.dismiss_overlays(cx);
        }
        if dismissed {
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

    /// Returns total number of leaves in the outer window layout tree.
    pub(crate) fn layout_leaf_count(&self) -> usize {
        self.panels.layout.tree.count_leaves()
    }
}
