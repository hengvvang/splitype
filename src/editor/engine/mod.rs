//! Editor engine: controller aggregate root, session management, and pane split layout.

pub mod controller;
pub(crate) mod file;
pub(crate) mod pane_layout;
pub mod session;
pub(crate) mod session_ops;

pub use controller::Editor;
pub use session::EditorSession;
