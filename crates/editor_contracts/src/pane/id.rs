#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PaneId(pub splitter::tree::NodeId);

impl From<splitter::tree::NodeId> for PaneId {
    #[inline]
    fn from(id: splitter::tree::NodeId) -> Self {
        Self(id)
    }
}

impl From<PaneId> for splitter::tree::NodeId {
    #[inline]
    fn from(id: PaneId) -> Self {
        id.0
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
