//! Navigation seam for the floating outline HUD.

use gpui::{App, Window};

/// Navigation seam: clicking a heading in the HUD asks the coordinating
/// crate to move the active pane to that heading.
pub trait OutlineHost: Send + Sync + 'static {
    /// Navigate to the heading at `index`.
    fn navigate_to(&self, index: usize, cx: &mut App);

    /// Report popover hover-state changes (debounced by the host). The
    /// window is passed so the host can schedule its debounce through a
    /// try-borrow-safe handle.
    fn set_hovered(&self, hovered: bool, window: &mut Window, cx: &mut App);
}
