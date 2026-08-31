use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Strongly-typed, extensible identifier of an editor pane kind.
///
/// Owned and hashable so it can come from plugin manifests and persisted
/// layouts, not only from compile-time literals. Built-in kinds use the
/// `splitype.pane.*` namespace going forward; legacy single-word names are
/// transitional.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaneKind(Arc<str>);

impl PaneKind {
    #[inline]
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Builds a kind from a `'static` string without allocating.
    #[inline]
    pub fn from_static(id: &'static str) -> Self {
        Self(Arc::from(id))
    }

    /// Reserved placeholder for sessions materialized before pane
    /// registration; never produced by a registered descriptor.
    pub fn unset() -> Self {
        Self::new("__splitype_unset__")
    }
}

impl Default for PaneKind {
    fn default() -> Self {
        Self::unset()
    }
}

impl fmt::Display for PaneKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
