//! `window` — the window-level microkernel container and pluggable panel SPI.

pub mod actions;
pub mod builder;
pub mod icons;
pub mod layout;
pub mod panel;
pub mod persist;

pub use actions::{
    ClosePanel, Copy, Cut, DismissTransientUi, OpenPath, OpenPathInSplit, Paste, SplitPanel,
    ToggleKindDropdown, TogglePanelMaximized, UpdateOpenTabPaths,
};
pub use builder::WindowLayoutBuilder;
pub use icons::{border_menu_style, panel_topbar_icon};
pub use layout::WindowLayout;
pub use panel::{MissingPanelView, PanelRegistry};
pub use persist::{PersistedPanel, PersistedWindowState, WINDOW_STATE_VERSION};
