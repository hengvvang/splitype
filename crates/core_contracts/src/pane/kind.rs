/// The strongly-typed, extensible identifier of an editor pane kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaneKind(pub &'static str);

impl PaneKind {
    #[inline]
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        self.0
    }

    #[inline]
    pub const fn empty() -> Self {
        Self("")
    }
}

impl Default for PaneKind {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::fmt::Display for PaneKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
