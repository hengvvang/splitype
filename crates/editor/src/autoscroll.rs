//! Autoscroll intent shared by every pane mode and the coordination layer.

use gpui::Pixels;

/// Algebraic autoscroll intent (mirrors Zed's AutoscrollStrategy).
///
/// Owned by the contract crate so the `PaneHost` seam can name it without
/// depending on any mode crate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoscrollStrategy {
    /// Scroll the minimal amount necessary to bring the active block / caret into view (with safe margin).
    Fit { margin: Pixels },
    /// Vertically center the active block / caret in the viewport.
    Center,
    /// Align the target block near the top of the viewport.
    Top { margin: Pixels },
    /// Align the target block near the bottom of the viewport.
    #[allow(dead_code)]
    Bottom { margin: Pixels },
}
