//! Panel view trait contract and rendering context.

use std::any::Any;
use std::path::Path;
use gpui::{AnyElement, App, FocusHandle, SharedString, Window};
use theme::Theme;
use config::language::I18nStrings;
use crate::layout::PanelId;
use crate::panel::PanelKind;

/// Rendering context passed to a [`PanelView`] on every render frame.
pub struct PanelRenderContext<'a> {
    pub panel_id: PanelId,
    pub leaf_count: usize,
    pub is_maximized: bool,
    pub theme: &'a Theme,
    pub strings: &'a I18nStrings,
}

/// Universal trait contract that any top-level window panel must implement.
pub trait PanelView: 'static {
    /// The unique identifier of this panel's kind.
    fn kind(&self) -> PanelKind;

    /// The human-readable display name shown in tabs, topbars or dropdown menus.
    fn display_name(&self) -> SharedString;

    /// The icon asset path for this panel (if any).
    fn icon(&self) -> Option<&'static str> {
        None
    }

    /// Whether this panel currently has unsaved modifications.
    fn is_dirty(&self, _cx: &App) -> bool {
        false
    }

    /// Title of the first unsaved document/item in this panel, if any.
    fn first_dirty_title(&self, _cx: &App) -> Option<String> {
        None
    }

    /// Save modifications in this panel.
    fn save(&mut self, _window: &mut Window, _cx: &mut App) -> Result<(), String> {
        Ok(())
    }

    /// Save modifications to a new location.
    fn save_as(&mut self, _window: &mut Window, _cx: &mut App) {}

    /// Query whether this panel can be closed safely.
    fn can_close(&self, _cx: &App) -> bool {
        true
    }

    /// Callback when this panel's activation state changes.
    fn on_active_changed(&mut self, _is_active: bool, _cx: &mut App) {}

    /// Callback when a filesystem path is modified, renamed, or deleted.
    fn on_fs_change(&mut self, _target_path: Option<&Path>, _cx: &mut App) {}

    /// Callback when a filesystem path is renamed or moved from one path to another.
    fn on_fs_path_renamed(&mut self, _from: &Path, _to: &Path, _cx: &mut App) {}

    /// Renders the complete panel UI inside the window tile container.
    fn render(
        &mut self,
        ctx: &PanelRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement;

    /// Updates the panel identifier owned by this view.
    fn set_panel_id(&mut self, _id: PanelId, _cx: &mut App) {}

    /// Discards unsaved changes in this panel.
    fn discard_changes(&mut self, _cx: &mut App) {}

    /// Save all dirty tabs/items in this panel.
    fn save_all(&mut self, window: &mut Window, cx: &mut App) -> Result<(), String> {
        self.save(window, cx)
    }

    /// The FocusHandle owned by this panel for keyboard navigation.
    fn focus_handle(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    /// Upcast to Any for reflection when necessary.
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

