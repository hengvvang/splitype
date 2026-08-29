#![recursion_limit = "2048"]
//! `app` — the composition root of splitype.
//!
//! Owns the window shell (`app::shell`), the keybinding installation
//! (`keybindings`), the bootstrap/CLI/platform glue, and the export
//! flow. The editor family crates (`editor_scheduler`, `editor_model`,
//! `editor_wysiwyg`, `editor_source_code`, `editor_preview`,
//! `editor_search`, `editor_outline`) and the panel crates
//! (`explorer`, `settings`) are all standalone; this crate wires them
//! together.

pub mod app;
pub mod keybindings;

pub mod platform;
