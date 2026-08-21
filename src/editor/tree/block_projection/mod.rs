//! Inline projection engine — editable Markdown delimiters on a block.
//!
//! Submodules organize lifecycle, 3D coordinate offset translations, and
//! projected text/heading/link mutations.

pub(crate) mod edits;
pub(crate) mod lifecycle;
pub(crate) mod offsets;
