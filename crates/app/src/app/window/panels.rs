//! Window panel state aggregate.
//!
//! The panel-kind vocabulary, panel ids, and the window-level split root
//! live in `workspace`; this module owns the window layout tree held by
//! the Shell. The explorer state and the settings panel state are gpui
//! `Global`s in their own crates and are not owned here.

use window::WindowLayout;

/// Sidebar and tiled-layout state of the window.
pub struct WindowPanels {
    pub(crate) layout: WindowLayout,
}

impl Default for WindowPanels {
    fn default() -> Self {
        let left_id = window::PanelId(1);
        let right_id = window::PanelId(2);
        let mut builder = window::WindowBuilder::new().with_split(
            left_id,
            window::PanelKind::new("explorer"),
            right_id,
            window::PanelKind::new("editor"),
            0.3,
            right_id,
        );
        Self {
            layout: builder.take_layout().unwrap(),
        }
    }
}

