//! Editor-level selection spanning multiple rendered blocks.

pub(crate) mod actions;
pub(crate) mod mouse_drag;
pub(crate) mod source_mutation;
pub(crate) mod state;

#[cfg(test)]
mod tests;
