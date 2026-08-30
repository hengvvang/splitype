//! editor_wysiwyg — the complete WYSIWYG Markdown editing engine plugin.
//!
//! Implements [`editor_model::PaneView`] to provide a rich visual block editor
//! with dual-direction text projection, syntax highlighting, LaTeX math,
//! Mermaid diagrams, native table editing, and Outline/Search capabilities.

pub mod export;
pub mod input;
pub mod model;
pub mod net;
pub mod plugin;
pub mod projection;
pub mod render;
pub mod syntax;
pub mod table;

// Public re-exports
pub use model::*;
pub use net::install_http_client;
pub use plugin::actions;
pub use plugin::actions::*;
pub use plugin::outline;
pub use plugin::outline::*;
pub use plugin::search;
pub use plugin::search::*;
pub use plugin::state;
pub use plugin::state::*;
pub use plugin::WysiwygPaneState;
pub use syntax::language as code_language;
pub use syntax::{highlight, language, latex, markdown, mermaid};
pub use table::*;

// Aliases for compatibility across internal submodules
pub use input::events;
pub use input::history;
pub use input::paste;
pub use input::paste::plain as paste_plain;
pub use input::selection;
pub use model as document;
pub use model::tree;
pub use plugin as pane;
pub use projection::source_map;
pub use render::presentation;
pub use render::text_layout;
pub use syntax::highlight as highlight_mod;
pub use syntax::latex as latex_mod;
pub use syntax::mermaid as mermaid_mod;
pub use table as table_grid;
pub use table::measure as table_measure;
