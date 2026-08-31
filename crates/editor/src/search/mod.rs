//! In-editor search and replace subsystem.
//!
//! The pure engine (`SearchQuery`/`RawMatch`), the panel state types, the
//! panel presentation and the input element live in the `core_contracts`
//! crate; the editor-facing coordination (query execution against the
//! document, highlight sync, jumping, replacing) stays here in `engine`
//! and `ime` (the `EntityInputHandler` impl must live next to the
//! `Editor` entity), plus the render shell in `render`.

pub mod engine;
pub mod ime;
pub mod render;
