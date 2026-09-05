//! `window` — the window-level microkernel container and pluggable panel SPI.

pub mod builder;
pub mod layout;
pub mod panel;
pub mod persist;

pub use builder::WindowLayoutBuilder;
pub use layout::WindowLayout;
pub use panel::{MissingPanelView, PanelRegistry};
pub use persist::{PersistedPanel, PersistedWindowState, WINDOW_STATE_VERSION};
