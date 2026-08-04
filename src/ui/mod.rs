//! Reusable UI components without business dependencies.
//!
//! Component code must not import `editor`, `model`, or `windows`; it only
//! consumes `theme` and gpui so any window can reuse it.

pub mod components;
