//! Command contribution contract — the vocabulary plugins use to declare
//! the actions they expose to menus and shortcuts.

pub mod registry;

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub use registry::{CommandContribution, CommandRegistry, CommandRegistryError};

/// Strongly-typed identifier of a contributed command.
///
/// Full command ids are namespaced by plugin (`<plugin-id>.<command>`, e.g.
/// `splitype.editor.save`) so contributions never collide across plugins.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandId(Arc<str>);

impl CommandId {
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
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
