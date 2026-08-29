#![recursion_limit = "2048"]
//! `splitype` — The fast, block-based Markdown editor with tiled split-views built with GPUI.

pub use splitype_core as core;
pub use splitype_infra as infra;
pub use splitype_model as model;
pub use splitype_render as render;
pub use splitype_splitter as splitter;
pub use splitype_ui as ui;

pub mod app;
pub mod editor;
pub mod explorer;
pub mod platform;
pub mod settings;
