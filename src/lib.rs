//! splitype — a block-based Markdown editor built with GPUI.
//!
//! Reads file paths from command-line arguments and opens one GPUI window per
//! file. With no arguments, a single empty window is created.
//!
//! # Crate layout
//!
//! - `model` — pure Markdown domain layer (no crate-internal imports);
//!   lives in the `splitype-model` crate and is re-exported here.
//! - `layout` — pure tiled-layout engine; lives in `splitype-layout`.
//! - `infra` — system capabilities (config, i18n, net, theme); lives in
//!   `splitype-infra`.
//! - `editor` — the editing runtime and window views.
//! - `explorer` / `settings` / `titlebar` — top-level views over `editor`.
//! - `app` — assembly: bootstrap, CLI, menus, window routing.
//! - `ui` — reusable components; `platform` — OS adapters.
//!
//! The domain (`model`), layout engine, and system capabilities (`infra`)
//! are separate crates so the dependency direction is enforced at compile
//! time: they cannot depend on anything in this crate.

pub use splitype_infra as infra;
pub use splitype_layout as layout;
pub use splitype_model as model;

pub mod app;
pub mod editor;
pub mod explorer;
pub mod platform;
pub mod settings;
pub mod titlebar;
pub mod ui;
