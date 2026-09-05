//! Window layout tree — the window-level split layout root.

use platform_contracts::PanelKind;
use splitter::root::SplitterRoot;

/// The window-level split layout root containing PanelKind tiles.
pub type WindowLayout = SplitterRoot<PanelKind>;
