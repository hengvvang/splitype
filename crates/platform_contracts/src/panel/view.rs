//! Panel view trait contract and rendering context.

use crate::panel::{PanelId, PanelKind};
use config::language::I18nStrings;
use gpui::{AnyElement, App, Bounds, Pixels, Point, SharedString, Window};
use std::any::Any;
use std::path::Path;
use theme::Theme;

/// Rendering context passed to a [`PanelView`] on every render frame.
pub struct PanelRenderContext<'a> {
    pub panel_id: PanelId,
    pub leaf_count: usize,
    pub is_maximized: bool,
    pub is_active: bool,
    /// Bounds of this panel's tile within the window, when laid out.
    pub bounds: Option<Bounds<Pixels>>,
    pub theme: &'a Theme,
    pub strings: &'a I18nStrings,
}

/// Universal trait contract that any top-level window panel must implement.
///
/// Optional roles (document routing, file trees, …) are opt-in and live in
/// the plugins that provide them; each plugin exports adapter functions that
/// the composition root registers by kind. The shell never downcasts to
/// concrete plugin types.
pub trait PanelView: 'static {
    /// The unique identifier of this panel's kind.
    fn kind(&self) -> PanelKind;

    /// The human-readable display name shown in tabs, topbars or dropdown menus.
    fn display_name(&self) -> SharedString;

    /// The icon asset path for this panel (if any).
    fn icon(&self) -> Option<&'static str> {
        None
    }

    /// Renders a window-level overlay for this panel (menus, popovers), if
    /// any. The shell draws these above the tiled layout so they are not
    /// clipped by the panel tile.
    fn render_overlay(&mut self, _window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        None
    }

    /// Dismisses this panel's transient overlays. Returns `true` when
    /// something was actually dismissed, so the shell can repaint.
    fn dismiss_overlays(&mut self, _cx: &mut App) -> bool {
        false
    }

    /// Esc dismissal, invoked only from the shell's global
    /// `DismissTransientUi` action handling. Cancels in-progress
    /// panel-internal split operations (drag gestures, border menus, kind
    /// dropdowns). Returns `true` when something was dismissed.
    ///
    /// Deliberately separate from [`PanelView::dismiss_overlays`], which
    /// also runs on every body mouse-down (click-away): drags must never
    /// be cancelled by the same event that just started them.
    fn handle_dismiss_transient_ui(&mut self, _cx: &mut App) -> bool {
        false
    }

    /// Whether this panel currently has unsaved modifications.
    fn is_dirty(&self, _cx: &App) -> bool {
        false
    }

    /// Title of the first unsaved document/item in this panel, if any.
    fn first_dirty_title(&self, _cx: &App) -> Option<String> {
        None
    }

    /// Save all dirty tabs/items in this panel. Panels without durable
    /// documents keep the default no-op success.
    fn save_all(&mut self, _window: &mut Window, _cx: &mut App) -> Result<(), String> {
        Ok(())
    }

    /// Callback when a filesystem path is renamed or moved from one path to another.
    fn on_fs_path_renamed(&mut self, _from: &Path, _to: &Path, _cx: &mut App) {}

    /// Handle a pointer move over the panel body, returning true when the
    /// panel consumed the event and needs a repaint.
    fn handle_inner_mouse_move(
        &mut self,
        _position: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> bool {
        false
    }

    /// Finish any in-progress panel-internal gesture.
    fn finish_inner_gestures(&mut self, _window: &mut Window, _cx: &mut App) {}

    /// Suspends this panel into an opaque state blob that can be handed back
    /// to [`PanelDescriptor::restore_panel`] later. Panels with durable
    /// documents return `Some` so their content survives kind switches.
    fn suspend_state(&mut self, _cx: &mut App) -> Option<Box<dyn Any>> {
        None
    }

    /// Clones the panel's durable state for a split or a cloned window.
    /// Returns `None` when the panel has no cloneable state.
    fn clone_state(&self, _cx: &mut App) -> Option<Box<dyn Any>> {
        None
    }

    /// Renders the complete panel UI inside the window tile container.
    fn render(&mut self, ctx: &PanelRenderContext, window: &mut Window, cx: &mut App)
    -> AnyElement;

    /// Updates the panel identifier owned by this view.
    fn set_panel_id(&mut self, _id: PanelId, _cx: &mut App) {}

    /// Discards unsaved changes in this panel.
    fn discard_changes(&mut self, _cx: &mut App) {}

    /// Upcast to Any for reflection when necessary.
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
