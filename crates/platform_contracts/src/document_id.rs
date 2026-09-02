//! Stable document identity shared across the platform and document layers.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identity of a document during its lifetime in the process.
///
/// Lives in the platform contract layer because both the window shell
/// (close guards, retained panel states) and the document layer (buffers,
/// snapshots) reference it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(Uuid);

impl DocumentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}
