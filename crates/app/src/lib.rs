//! `app` — the composition root of splitype.
//!
//! Owns the window shell ([`shell::Shell`]), the keybinding installation
//! ([`keybindings`]), the bootstrap/CLI/platform glue, the menu system ([`menus`]),
//! window chrome ([`chrome`]), layout ([`layout`]), and dialogs ([`dialogs`]).
//!
//! The modular editor engine (`editor`), contracts (`core_contracts`),
//! pane plugins (`pane_wysiwyg`, `pane_source_code`, `pane_preview`),
//! and panel crates (`explorer`, `settings`) are wired together here.

pub mod actions;
pub mod assets;
pub mod bootstrap;
pub mod chrome;
pub mod commands;
pub mod dialogs;
pub mod keybindings;
pub mod layout;
pub mod menus;
pub mod platform;
pub mod plugins;
pub mod shell;
pub mod window;
pub mod window_state;

pub use ::window::PanelKind;
