//! Editor window panels: the workspace sidebar and the tiled pane layout.

use crate::editor::window::layout::WindowLayout;
use crate::editor::window::workspace::Workspace;

/// Sidebar and tiled-layout state of the editor window.
///
/// Pure state records; rendering lives in `ui::window`.
#[derive(Default)]
pub struct WindowPanels {
    pub(crate) workspace: Workspace,
    pub(crate) layout: WindowLayout,
}
