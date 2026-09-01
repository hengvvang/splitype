//! Plugin contract vocabulary: identity, manifests, and the discovery record.
//!
//! The contracts here are neutral to how a plugin's code is provided
//! (statically linked today; WASM or subprocess transports later), so the
//! shell can treat every plugin uniformly.

pub mod id;
pub mod manifest;
pub mod registry;

pub use id::PluginId;
pub use manifest::{
    ManifestCommand, PLUGIN_MANIFEST_VERSION, PluginCapabilities, PluginEntry, PluginManifest,
    PluginManifestError, PluginResources,
};
pub use registry::{PluginRegistry, PluginRegistryError};
