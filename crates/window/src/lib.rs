//! `window` — the window-level microkernel container and pluggable panel SPI.

pub mod actions;
pub mod builder;
pub mod icons;
pub mod layout;
pub mod panel;

pub use actions::{
    ClosePanel, Copy, Cut, DismissTransientUi, OpenPath, OpenPathInSplit, Paste, SplitPanel,
    ToggleKindDropdown, TogglePanelMaximized, UpdateOpenTabPaths,
};
pub use builder::WindowBuilder;
pub use icons::{border_menu_style, panel_topbar_icon};
pub use layout::{PanelId, WindowLayout};
pub use panel::{
    DocumentPanel, PanelCapabilities, PanelDescriptor, PanelHost, PanelKind, PanelRegistry,
    PanelRenderContext, PanelView, SidebarPanel,
};
