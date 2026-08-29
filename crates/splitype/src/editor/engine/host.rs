//! EditorHost — the service contract between the editor family and the
//! window shell (dependency inversion seam).
//!
//! The editor never names the shell type: [`Editor`](crate::editor::engine::controller::Editor)
//! holds `Option<Arc<dyn EditorHost>>` and the app's composition root
//! injects a `ShellEditorHost` (defined next to `Shell`) when it spawns
//! editor entities. Every shell-side capability the editor needs goes
//! through this trait, so the editor crates depend on nothing above them
//! in the dependency graph and can be exercised with a no-op host in
//! tests. Window-scoped work that must run after an editor update finishes
//! is deferred via [`Editor::defer_host_action`]; the host itself never
//! re-enters the editor.

use std::path::Path;

use gpui::{App, Window};

use crate::editor::engine::session::OpenFileMode;
use splitter::SplitAxis;
use workspace::PanelId;

/// Service contract between the editor family and the window shell.
///
/// All methods take `&mut App` (never a shell context) so implementations
/// can be invoked from deferred app callbacks without naming the shell
/// type. Methods that need a `Window` receive it as an argument.
pub trait EditorHost: Send + Sync + 'static {
    /// Bring the window panel `panel_id` to the foreground.
    fn activate_panel(&self, panel_id: PanelId, cx: &mut App);

    /// Toggle the window panel's kind dropdown (top bar control).
    fn toggle_panel_dropdown(&self, panel_id: PanelId, cx: &mut App);

    /// Split the window panel into two editor panels along `axis`.
    fn split_panel(
        &self,
        panel_id: PanelId,
        axis: SplitAxis,
        ratio: f32,
        copy_content: bool,
        cx: &mut App,
    );

    /// Maximize or restore the window panel.
    fn toggle_panel_maximize(&self, panel_id: PanelId, cx: &mut App);

    /// Request closing the window panel (runs the shell's dirty check).
    fn request_close_panel(&self, panel_id: PanelId, cx: &mut App);

    /// Prompt the shell's unsaved-changes dialog for one tab.
    fn prompt_close_tab(&self, panel_id: PanelId, index: usize, cx: &mut App);

    /// Open `path` in the active editor tab of the shell.
    fn open_file_in_active_editor(
        &self,
        path: &Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut App,
    ) -> bool;

    /// Dismiss the shell's info dialog (drop-replace flow).
    fn hide_info_dialog(&self, cx: &mut App);

    /// Close window-level layout dropdowns opened by the shell.
    fn clear_outer_dropdowns(&self, cx: &mut App);

    /// Keep the explorer selection in sync after a document path change
    /// (the explorer is a sibling panel; the editor must not name it).
    fn sync_explorer_after_document_path_change(&self, cx: &mut App);
}
