// ── Core engine modules ────────────────────────────────────────────────────
pub(crate) mod document;
pub(crate) mod editor;
pub mod history;
pub(crate) mod selection;
pub(crate) mod source_map;
pub(crate) mod table_edit;
#[cfg(test)]
pub(crate) mod tests;
pub(crate) mod viewport;

// ── Re-exports ─────────────────────────────────────────────────────────────
pub(crate) use editor::{EditMode, RenderedSelectAllCycle};
