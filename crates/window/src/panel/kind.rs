//! Window panel kind identifier.

use std::fmt;
use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Strongly-typed, extensible identifier for a window-level panel.
///
/// Owned and hashable so it can come from plugin manifests and persisted
/// layouts, not only from compile-time literals. Built-in kinds use the
/// `splitype.panel.*` namespace going forward; legacy single-word names are
/// transitional.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PanelKind(Arc<str>);

impl PanelKind {
    #[inline]
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PanelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for PanelKind {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PanelKind {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        Ok(Self(Arc::from(id)))
    }
}

impl JsonSchema for PanelKind {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PanelKind".into()
    }

    fn json_schema(r#gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        String::json_schema(r#gen)
    }
}
