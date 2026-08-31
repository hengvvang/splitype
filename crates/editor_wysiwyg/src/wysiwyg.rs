//! editor_wysiwyg — the complete WYSIWYG Markdown editing engine plugin.
//!
//! Implements [`editor_model::PaneView`] to provide a rich visual block editor
//! with dual-direction text projection, syntax highlighting, LaTeX math,
//! Mermaid diagrams, native table editing, and Outline/Search capabilities.

pub mod builder;
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
pub use builder::*;
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
pub use render::presentation;
pub use syntax::language as code_language;
pub use syntax::{highlight, language, latex, markdown, mermaid};
pub use table::*;


