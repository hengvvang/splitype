//! Inline filename editor for the explorer: buffer operations, validation,
//! IME bridge, and custom GPUI input element.

pub(crate) mod buffer;
pub(crate) mod element;
pub(crate) mod ime;
pub(crate) mod operations;

pub(crate) use element::ExplorerFilenameInputElement;
