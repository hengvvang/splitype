//! Reusable UI components without business dependencies.
//!
//! Component code must not import `editor`, `model`, `explorer`, `settings`,
//! `titlebar`, or `app`; it only consumes `infra::theme` and gpui so any
//! window can reuse it.

pub mod components;
