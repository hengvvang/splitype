//! Pane plugin registry — the central registry for editor pane descriptors and factories.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use gpui::SharedString;

use crate::{PaneKindId, PaneView};

/// Descriptor and factory for a pane plugin.
pub trait PaneDescriptor: Send + Sync + 'static {
    /// The unique identifier of this pane kind.
    fn kind(&self) -> PaneKindId;

    /// User-facing display name shown in tab/mode selector.
    fn display_name(&self) -> SharedString;

    /// Optional icon path for the pane mode.
    fn icon_path(&self) -> Option<SharedString> {
        None
    }

    /// Create a fresh pane view instance.
    fn create_pane(&self) -> Box<dyn PaneView>;
}

/// App-wide pane registry.
pub struct PaneRegistry {
    descriptors: HashMap<PaneKindId, Arc<dyn PaneDescriptor>>,
    order: Vec<PaneKindId>,
}

impl PaneRegistry {
    /// The process-wide registry.
    pub fn global() -> &'static Mutex<PaneRegistry> {
        static REGISTRY: LazyLock<Mutex<PaneRegistry>> = LazyLock::new(|| {
            Mutex::new(PaneRegistry {
                descriptors: HashMap::new(),
                order: Vec::new(),
            })
        });
        &REGISTRY
    }

    /// Register a pane descriptor.
    pub fn register(&mut self, descriptor: Arc<dyn PaneDescriptor>) {
        let kind = descriptor.kind();
        if !self.descriptors.contains_key(&kind) {
            self.order.push(kind);
        }
        self.descriptors.insert(kind, descriptor);
    }

    /// Retrieve the descriptor for `kind`.
    pub fn get(&self, kind: PaneKindId) -> Option<Arc<dyn PaneDescriptor>> {
        self.descriptors.get(&kind).cloned()
    }

    /// Create a fresh pane instance for `kind`.
    pub fn create(&self, kind: PaneKindId) -> Box<dyn PaneView> {
        self.descriptors
            .get(&kind)
            .unwrap_or_else(|| panic!("no pane descriptor registered for {kind}"))
            .create_pane()
    }

    /// All registered pane descriptors in order of registration.
    pub fn all_descriptors(&self) -> Vec<Arc<dyn PaneDescriptor>> {
        self.order
            .iter()
            .filter_map(|kind| self.descriptors.get(kind).cloned())
            .collect()
    }

    /// Default pane kind.
    pub fn default_kind(&self) -> PaneKindId {
        self.order.first().copied().unwrap_or(PaneKindId::WYSIWYG)
    }
}
