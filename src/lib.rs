#![recursion_limit = "2048"]
//! splitype — a block-based Markdown editor built with GPUI.
//!
//! Reads file paths from command-line arguments and opens one GPUI window per
//! file. With no arguments, a single empty window is created.
//!
//! # Crate layout
//!
//! - `model` — pure Markdown domain layer; lives in the `splitype-model`
//!   crate and is re-exported here.
//! - `splitter` — the tiled split-screen engine; lives in `splitype-splitter`.
//! - `infra` — system capabilities (config, i18n, net, theme).
//! - `editor` — the editing runtime and window views.
//! - `explorer` / `settings` — top-level views over `editor`.
//! - `app` — assembly: bootstrap, CLI, menus, window routing.
//! - `ui` — reusable components; `platform` — OS adapters.
//!
//! The domain (`model`) and the splitter engine stay in separate crates because
//! they are reusable engines with no application coupling. Everything that
//! is specific to this application (`infra`, `ui`, `platform`) lives here
//! as plain modules so the boundary between engine and application stays
//! obvious.

pub use splitype_model as model;
pub use splitype_splitter as splitter;

pub mod app;
pub mod editor;
pub mod explorer;
pub mod infra;
pub mod platform;
pub mod settings;
pub mod ui;
