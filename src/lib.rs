//! splitype — a block-based Markdown editor built with GPUI.
//!
//! Reads file paths from command-line arguments and opens one GPUI window per
//! file. With no arguments, a single empty window is created.
//!
//! # Crate layout
//!
//! - `model` — pure Markdown domain layer (no crate-internal imports).
//! - `layout` — pure tiled-layout engine.
//! - `infra` — system capabilities (config, i18n, net, theme).
//! - `editor` — the editing runtime and window views.
//! - `explorer` / `settings` / `titlebar` — top-level views over `editor`.
//! - `app` — assembly: bootstrap, CLI, menus, window routing.
//! - `ui` — reusable components; `platform` — OS adapters.
//!
//! The library target exists so benches and integration tests (`tests/`)
//! can exercise the real API instead of re-implementing internals.

pub mod app;
pub mod editor;
pub mod explorer;
pub mod infra;
pub mod layout;
pub mod model;
pub mod platform;
pub mod settings;
pub mod titlebar;
pub mod ui;
