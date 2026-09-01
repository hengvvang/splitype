//! Plugin identity — the stable reverse-domain identifier of a plugin.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Strongly-typed, extensible identifier for a plugin.
///
/// Plugins use reverse-domain ids (e.g. `splitype.editor`,
/// `com.vendor.product`) so manifests, kinds, and `plugin://` resource URLs
/// share one vocabulary. Owned and hashable so ids can come from manifests.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginId(Arc<str>);

impl PluginId {
    #[inline]
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// Builds an id from a `'static` string without allocating.
    #[inline]
    pub fn from_static(id: &'static str) -> Self {
        Self(Arc::from(id))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the id has a reverse-domain shape (`vendor.product`).
    pub fn is_namespaced(&self) -> bool {
        self.0.split('.').count() >= 2
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
