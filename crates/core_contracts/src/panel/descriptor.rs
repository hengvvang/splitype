//! Panel descriptor trait for registering panel plugins.

use crate::panel::{PanelCapabilities, PanelHost, PanelId, PanelKind, PanelView};
use gpui::{App, SharedString};
use std::any::Any;
use std::sync::Arc;

/// Factory descriptor for a Panel plugin.
pub trait PanelDescriptor: Send + Sync + 'static {
    /// The unique kind identifier.
    fn kind(&self) -> PanelKind;

    /// Human-readable display name.
    fn display_name(&self) -> SharedString;

    /// Icon path for this panel kind.
    fn icon(&self) -> Option<&'static str> {
        None
    }

    /// The roles panels of this kind can serve. Hosts consult this before
    /// instantiation (e.g. to build the default layout or decide activation
    /// policy) without comparing kind strings.
    fn capabilities(&self) -> PanelCapabilities {
        PanelCapabilities::default()
    }

    /// Instantiates a new PanelView for a given PanelId.
    fn create_panel(
        &self,
        panel_id: PanelId,
        host: Arc<dyn PanelHost>,
        cx: &mut App,
    ) -> Box<dyn PanelView>;

    /// Rebuilds a panel from a state previously returned by
    /// [`PanelView::suspend_state`] or [`PanelView::clone_state`].
    ///
    /// Returns `None` when the state does not belong to this descriptor.
    fn restore_panel(
        &self,
        panel_id: PanelId,
        host: Arc<dyn PanelHost>,
        state: Box<dyn Any>,
        cx: &mut App,
    ) -> Option<Box<dyn PanelView>> {
        let _ = (panel_id, host, state, cx);
        None
    }

    /// Dirty summary for suspended state of this panel kind. The shell uses
    /// this to protect documents that survive a kind switch without a live
    /// panel view.
    fn retained_dirty_info(&self, _state: &dyn Any, _cx: &App) -> (bool, Option<String>) {
        (false, None)
    }

    /// Discards unsaved changes held inside suspended state of this panel kind.
    fn discard_retained(&self, _state: &mut Box<dyn Any>, _cx: &mut App) {}

    /// Serializes a state blob (from [`PanelView::suspend_state`] or
    /// [`PanelView::clone_state`]) for window-state persistence.
    ///
    /// Panels that do not opt in return `None` and are recreated fresh when
    /// the window state is restored.
    fn serialize_state(&self, _state: &dyn Any) -> Option<serde_json::Value> {
        None
    }

    /// Rebuilds a state blob from persisted window-state JSON.
    fn deserialize_state(&self, _json: &serde_json::Value) -> Option<Box<dyn Any>> {
        None
    }
}
