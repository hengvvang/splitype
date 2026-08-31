//! Window-shell contract — the window-level vocabulary shared by the app
//! shell (`crates/app`), the editor family, and the sidebar panels
//! (`explorer`, `settings`).
//!
//! Owns:
//! - [`PanelId`] / [`WindowPanelKind`] / [`EditorPanelMode`] — window panel
//!   identity and kind vocabulary (the editor-internal pane vocabulary
//!   lives in `crates/editor`);
//! - the default window layout ([`default_layout`]) and its constants;
//! - window-level layout actions ([`actions`]) that panels dispatch and the
//!   shell handles;
//! - shared window-chrome presentation helpers ([`icons`]).
//!
//! This crate must stay free of editor-family and panel dependencies so
//! every consumer can depend on it without cycles. The `Shell` entity
//! itself lives in `crates/app`.

pub mod actions;
pub mod builder;
pub mod icons;
pub mod panels;
pub mod plugin;

pub use actions::{
    ClosePanel, Copy, Cut, DismissTransientUi, OpenInEditor, OpenInSplit, Paste, SplitPanel,
    ToggleKindDropdown, TogglePanelMaximized,
};
pub use builder::*;
pub use icons::{border_menu_style, panel_topbar_icon};
pub use panels::{
    EditorPanelMode, PanelId, WindowPanelKind, WindowLayout, default_layout,
    DEFAULT_EDITOR_PANEL_ID, ROOT_PANEL_ID,
};
pub use plugin::*;
