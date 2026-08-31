//! Host interface provided by the window shell to panel instances.

use crate::panel::{PanelId, PanelKind};
use gpui::App;
use splitter::tree::SplitAxis;

/// Host interface allowing any panel plugin to request window-level operations.
pub trait PanelHost: Send + Sync + 'static {
    /// Request focus for this panel.
    fn activate_panel(&self, panel_id: PanelId, cx: &mut App);

    /// Close this panel tile.
    fn close_panel(&self, panel_id: PanelId, cx: &mut App);

    /// Split this panel along the specified axis.
    fn split_panel(
        &self,
        panel_id: PanelId,
        axis: SplitAxis,
        new_kind: Option<PanelKind>,
        cx: &mut App,
    );

    /// Toggle maximized state of this panel tile.
    fn toggle_maximize(&self, panel_id: PanelId, cx: &mut App);
}
