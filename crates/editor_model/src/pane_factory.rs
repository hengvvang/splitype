//! Pane factory registry — the dependency-inversion seam between the
//! editor contract and the mode crates.
//!
//! `editor` never names a mode type; the app composition root registers
//! one [`PaneFactory`] per [`EditorPaneKind`] at startup and the editor
//! creates pane states through the registry. Mode crates only implement
//! [`Pane`]; they never reference editor internals.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::{EditorPaneKind, Pane};

/// Creates a fresh pane state for a kind.
pub trait PaneFactory: Send + Sync + 'static {
    fn new_pane(&self, kind: EditorPaneKind) -> Box<dyn Pane>;
}

/// App-wide pane factory registry.
pub struct PaneFactoryRegistry {
    factories: HashMap<EditorPaneKind, Box<dyn PaneFactory>>,
}

impl PaneFactoryRegistry {
    /// The process-wide registry (no context needed, so pane states can
    /// be created from plain constructors).
    pub fn global() -> &'static Mutex<PaneFactoryRegistry> {
        static REGISTRY: LazyLock<Mutex<PaneFactoryRegistry>> = LazyLock::new(|| {
            Mutex::new(PaneFactoryRegistry {
                factories: HashMap::new(),
            })
        });
        &REGISTRY
    }

    /// Register the factory for `kind` (called by the composition root).
    pub fn register(&mut self, kind: EditorPaneKind, factory: Box<dyn PaneFactory>) {
        self.factories.insert(kind, factory);
    }

    /// Create a fresh pane state for `kind`.
    ///
    /// Panics when no factory was registered (the composition root must
    /// install all factories before any editor is created).
    pub fn create(&self, kind: EditorPaneKind) -> Box<dyn Pane> {
        self.factories
            .get(&kind)
            .unwrap_or_else(|| panic!("no pane factory registered for {kind:?}"))
            .new_pane(kind)
    }
}
