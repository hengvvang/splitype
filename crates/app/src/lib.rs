//! `app` — the composition root of splitype.
//!
//! Owns the window shell (`app::shell`), the keybinding installation
//! (`keybindings`), the bootstrap/CLI/platform glue, and the export
//! flow. The modular editor engine (`editor`), contracts (`core_contracts`),
//! pane plugins (`pane_wysiwyg`, `pane_source_code`, `pane_preview`),
//! and panel crates (`explorer`, `settings`) are wired here together.

pub mod app;
pub mod keybindings;

pub mod platform;
