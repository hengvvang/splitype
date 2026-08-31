use crate::document::TabKind;
use crate::panel::PanelId;
use gpui::{App, Window};
use std::path::Path;

/// Host contract handed by the window shell to document-routing panels.
///
/// The shell implements this once and every [`crate::DocumentPanel`] can
/// call back through it; the contract carries no editor-specific notions, so
/// alternative document plugins receive the same service.
pub trait DocumentHost: Send + Sync + 'static {
    fn activate_panel(&self, panel_id: PanelId, cx: &mut App);
    fn toggle_panel_dropdown(&self, panel_id: PanelId, cx: &mut App);
    fn split_panel(
        &self,
        panel_id: PanelId,
        axis: splitter::SplitAxis,
        ratio: f32,
        copy_content: bool,
        cx: &mut App,
    );
    fn toggle_panel_maximize(&self, panel_id: PanelId, cx: &mut App);
    fn request_close_panel(&self, panel_id: PanelId, cx: &mut App);
    fn prompt_close_tab(&self, panel_id: PanelId, index: usize, cx: &mut App);
    fn open_file_in_active_document_panel(
        &self,
        path: &Path,
        kind: TabKind,
        window: &mut Window,
        cx: &mut App,
    ) -> bool;
    fn hide_info_dialog(&self, cx: &mut App);
    fn clear_outer_dropdowns(&self, cx: &mut App);
    /// A document's backing path changed (saved-as or drop replacement);
    /// the shell decides which window-level services need to react.
    fn on_document_path_changed(&self, cx: &mut App);
    fn record_recent_file(&self, path: &Path, cx: &mut App);
}
