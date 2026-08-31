//! Window layout tree and panel identifiers.

use crate::panel::PanelKind;
pub use core_contracts::PanelId;
use splitter::root::SplitterRoot;

/// The window-level split layout root containing PanelKind tiles.
pub type WindowLayout = SplitterRoot<PanelKind>;
