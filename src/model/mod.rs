//! Pure Markdown domain model — data types, syntax parsers, and tree parsing.
//!
//! This layer has no GPUI entity or window dependencies. It may use GPUI's
//! plain data types (e.g. `SharedString`) but never `App`, `Window`, or
//! entity handles. Everything here is testable without a runtime.

pub mod block;
pub mod inline;
pub mod parse;
pub mod syntax;
