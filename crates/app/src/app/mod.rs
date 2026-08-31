//! Application shell — bootstrap, menus, actions, windows, and assets.
//! (CLI argument parsing lives in the `splitype_cli` crate; the
//! installer/update-check in `splitype_installer`.)

pub mod actions;
pub mod assets;
pub mod bootstrap;
pub mod menus;
pub(crate) mod shell;
pub mod window;

pub use workspace::PanelKindId;
