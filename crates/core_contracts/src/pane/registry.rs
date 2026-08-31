use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use crate::pane::{PaneDescriptor, PaneKind, PaneView};

pub struct PaneRegistry {
    descriptors: HashMap<PaneKind, Arc<dyn PaneDescriptor>>,
    order: Vec<PaneKind>,
    default_kind: Option<PaneKind>,
}

impl PaneRegistry {
    pub fn global() -> &'static Mutex<PaneRegistry> {
        static REGISTRY: LazyLock<Mutex<PaneRegistry>> = LazyLock::new(|| {
            Mutex::new(PaneRegistry {
                descriptors: HashMap::new(),
                order: Vec::new(),
                default_kind: None,
            })
        });
        &REGISTRY
    }

    pub fn register(&mut self, descriptor: Arc<dyn PaneDescriptor>, is_default: bool) {
        let kind = descriptor.kind();
        if !self.descriptors.contains_key(&kind) {
            self.order.push(kind);
        }
        if is_default || self.default_kind.is_none() {
            self.default_kind = Some(kind);
        }
        self.descriptors.insert(kind, descriptor);
    }

    pub fn get(&self, kind: PaneKind) -> Option<Arc<dyn PaneDescriptor>> {
        self.descriptors.get(&kind).cloned()
    }

    pub fn create(&self, kind: PaneKind) -> Option<Box<dyn PaneView>> {
        self.descriptors.get(&kind).map(|d| d.create_pane())
    }

    pub fn default_kind(&self) -> Option<PaneKind> {
        self.default_kind.or_else(|| self.order.first().copied())
    }

    pub fn all_descriptors(&self) -> Vec<Arc<dyn PaneDescriptor>> {
        self.order
            .iter()
            .filter_map(|kind| self.descriptors.get(kind).cloned())
            .collect()
    }
}
