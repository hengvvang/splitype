//! wysiwyg — the complete WYSIWYG Markdown editing engine plugin.
//!
//! Implements [`editor_contracts::PaneView`] to provide a rich visual block editor
//! with dual-direction text projection, syntax highlighting, LaTeX math,
//! Mermaid diagrams, native table editing, and Outline/Search capabilities.
//!
//! The pane's contract adapter lives in [`pane`] ([`pane::WysiwygPaneState`]
//! implements [`editor_contracts::PaneView`]).
//!
//! [`pane::WysiwygPaneState`]: pane::state::WysiwygPaneState

pub mod builder;
pub mod input;
pub mod model;
pub mod net;
pub mod pane;
pub mod projection;
pub mod render;
pub mod settings;
pub mod table;

pub use builder::*;
pub use settings::{ImagePasteBehavior, WysiwygSettings};
