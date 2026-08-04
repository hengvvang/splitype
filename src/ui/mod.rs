//! Reusable UI components without business dependencies.
//!
//! Component code must not import `editor`, `model`, or `windows`; it only
//! consumes `theme` and gpui so any window can reuse it.
//!
//! `blocks` and `inline` are being migrated to `windows::editor::blocks`.

pub mod blocks;
pub mod components;
pub mod inline;
