//! Bottom status bar hover state.
//!
//! Hover flags tracked across frames so bottombar widgets can render their
//! background without re-deriving state from element closures.

/// Mutable hover state tracked across frames for bottombar widgets.
#[derive(Default)]
pub(crate) struct BottombarState {
    pub sidebar_hovered: bool,
    pub mode_hovered: bool,
    pub custom_button_hovered: Option<String>,
}
