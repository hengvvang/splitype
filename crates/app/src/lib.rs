#![recursion_limit = "2048"]
//! `app` — the composition root of splitype.
//!
//! Owns the window shell (`app::shell`), the `Editor` entity and its
//! coordination layer (`editor`), the bootstrap/CLI/platform glue, and
//! the export flow. The editor family crates (`editor`,
//! `editor_wysiwyg`, `editor_source_code`, `editor_preview`,
//! `editor_search`, `editor_outline`) and the panel crates
//! (`explorer`, `settings`) are all standalone; this crate wires them
//! together.

pub mod app;
pub mod editor_scheduler;

pub mod platform;
