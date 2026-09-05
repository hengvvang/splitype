//! preview — the read-only rendered Markdown preview pane.
//!
//! Standard-first: the preview tree is built from the CommonMark parse
//! (100% CommonMark), unlike the WYSIWYG 1:1-line parser. The preview
//! owns its full presentation (block renderers, footnote section, quote
//! guides) and never touches WYSIWYG editing internals.
//!
//! The pane state implements [`editor_contracts::PaneView`]. The coordinating
//! crate only refreshes the tree, routes focus and hands over the scroll
//! shell through [`editor_contracts::PaneRenderContext`].

pub mod assets;
pub mod block;
pub mod builder;
mod context;
pub mod outline;
pub mod pane;
pub mod render;
pub mod search;
pub mod settings;
pub use builder::*;

pub const MANIFEST_TOML: &str = include_str!("../manifest.toml");
