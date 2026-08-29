//! Inline filename editor for the explorer: buffer operations, validation,
//! and the custom GPUI input element. (The `EntityInputHandler` IME bridge
//! for the shell lives in `crates/app` — orphan rule.)

pub mod buffer;
pub mod element;
pub(crate) mod operations;

pub mod ime;

pub use element::ExplorerFilenameInputElement;
pub use ime::ExplorerFilenameImeHost;
