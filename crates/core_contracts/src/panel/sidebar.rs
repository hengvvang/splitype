//! Optional sidebar role for window panels.
//!
//! A panel whose descriptor declares [`PanelCapabilities::sidebar`] should
//! also implement this trait so the shell can push document context and
//! sidebar commands to it without knowing its concrete type.

use crate::panel::PanelView;
use gpui::{App, Window};
use std::path::PathBuf;

/// The sidebar role of a [`PanelView`]: a companion panel that mirrors the
/// active document and offers drawer/folder-scope controls.
pub trait SidebarPanel: PanelView {
    /// The active document's path changed. The shell pushes this on every
    /// frame so the sidebar can sync its selection; `None` means no document
    /// is active.
    fn set_active_document_path(&mut self, path: Option<PathBuf>, cx: &mut App);

    /// A document's backing path changed (save-as or drop replacement).
    fn on_document_path_changed(&mut self, cx: &mut App);

    /// Toggle the sidebar's drawer visibility.
    fn toggle_drawer(&mut self, window: &mut Window, cx: &mut App);

    /// Close the currently open folder scope.
    fn close_active_folder(&mut self, cx: &mut App);
}
