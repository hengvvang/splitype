//! Generic layout types — the single node-id concept and the split
//! seeding mode.
//!
//! The engine is generic over the leaf kind; concrete kinds are registered
//! by the hosts (the application defines `WindowAreaKind`, the editor
//! defines `EditorInnerPanelKind`, both implementing `ContainerKind`).
//! Every node of every container — leaves and split nodes alike — is
//! numbered from one id space ([`NodeId`]), so nested containers stay
//! fully decoupled: hosts glue containers together with their own maps.

/// The one id concept of the engine: every node of every container
/// (leaves and split nodes alike) is numbered from this single space.
pub type NodeId = usize;

/// How the new sibling leaf of a split is seeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaSplitMode {
    /// The sibling inherits the source leaf's kind; hosts may deep-copy
    /// the source's content (the editor clones its inner panel layout and
    /// tab list).
    Copy,
    /// The sibling is a blank initial-state leaf of the same kind.
    Fresh,
}
