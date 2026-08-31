use std::path::Path;
use gpui::{App, Window};
use crate::document::OpenFileMode;

pub trait EditorHost: Send + Sync + 'static {
    fn activate_panel(&self, panel_id: window::PanelId, cx: &mut App);
    fn toggle_panel_dropdown(&self, panel_id: window::PanelId, cx: &mut App);
    fn split_panel(
        &self,
        panel_id: window::PanelId,
        axis: splitter::SplitAxis,
        ratio: f32,
        copy_content: bool,
        cx: &mut App,
    );
    fn toggle_panel_maximize(&self, panel_id: window::PanelId, cx: &mut App);
    fn request_close_panel(&self, panel_id: window::PanelId, cx: &mut App);
    fn prompt_close_tab(&self, panel_id: window::PanelId, index: usize, cx: &mut App);
    fn open_file_in_active_editor(
        &self,
        path: &Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut App,
    ) -> bool;
    fn hide_info_dialog(&self, cx: &mut App);
    fn clear_outer_dropdowns(&self, cx: &mut App);
    fn sync_explorer_after_document_path_change(&self, cx: &mut App);
    fn record_recent_file(&self, path: &Path, cx: &mut App);
}

