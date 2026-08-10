//! Generic layout types — the single node-id concept.
//!
//! The engine is generic over the leaf kind (`T` in [`SplitterContainer`]);
//! concrete kinds are defined by the hosts (the application defines
//! `WindowAreaKind`, the editor defines `EditorInnerPanelKind`). Every
//! node of every container — leaves and split nodes alike — is numbered
//! from one id space ([`NodeId`]), so nested containers stay fully
//! decoupled: hosts glue containers together with their own maps.

/// The one id concept of the engine: every node of every container
/// (leaves and split nodes alike) is numbered from this single space.
pub type NodeId = usize;
