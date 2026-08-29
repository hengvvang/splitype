#![recursion_limit = "2048"]
//! `splitype` — The fast, block-based Markdown editor with tiled split-views built with GPUI.

pub use primitives;
pub use sum_tree;
pub use markdown;
pub use splitter;
pub use theme;
pub use i18n;
pub use config;
pub use net;
pub use syntax;
pub use latex;
pub use mermaid;
pub use export;
pub use ui;

pub mod app;
pub mod editor;
pub mod explorer;
pub mod platform;
pub mod settings;
