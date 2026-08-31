use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use thiserror::Error;

use crate::pane::{PaneDescriptor, PaneKind, PaneView};

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PaneRegistryError {
    #[error("pane kind '{0}' is already registered")]
    DuplicateKind(PaneKind),
    #[error("pane registry lock is poisoned")]
    Poisoned,
}

#[derive(Default)]
pub struct PaneRegistry {
    descriptors: HashMap<PaneKind, Arc<dyn PaneDescriptor>>,
    order: Vec<PaneKind>,
    default_kind: Option<PaneKind>,
}

impl PaneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn global() -> &'static Mutex<Self> {
        static REGISTRY: LazyLock<Mutex<PaneRegistry>> =
            LazyLock::new(|| Mutex::new(PaneRegistry::new()));
        &REGISTRY
    }

    pub fn register(
        &mut self,
        descriptor: Arc<dyn PaneDescriptor>,
        is_default: bool,
    ) -> Result<(), PaneRegistryError> {
        // Query plugin metadata before mutating the registry. A descriptor callback
        // must never observe a half-completed registration.
        let kind = descriptor.kind();
        if self.descriptors.contains_key(&kind) {
            return Err(PaneRegistryError::DuplicateKind(kind));
        }

        self.order.push(kind);
        if is_default || self.default_kind.is_none() {
            self.default_kind = Some(kind);
        }
        self.descriptors.insert(kind, descriptor);
        Ok(())
    }

    pub fn register_global(
        descriptor: Arc<dyn PaneDescriptor>,
        is_default: bool,
    ) -> Result<(), PaneRegistryError> {
        Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .register(descriptor, is_default)
    }

    pub fn get(&self, kind: PaneKind) -> Option<Arc<dyn PaneDescriptor>> {
        self.descriptors.get(&kind).cloned()
    }

    pub fn registered(
        kind: PaneKind,
    ) -> Result<Option<Arc<dyn PaneDescriptor>>, PaneRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .get(kind))
    }

    pub fn create_registered(
        kind: PaneKind,
    ) -> Result<Option<Box<dyn PaneView>>, PaneRegistryError> {
        // Clone the descriptor while locked, then execute third-party factory code
        // only after the guard has been released. This permits safe re-entry.
        let descriptor = Self::registered(kind)?;
        Ok(descriptor.map(|descriptor| descriptor.create_pane()))
    }

    pub fn default_kind(&self) -> Option<PaneKind> {
        self.default_kind.or_else(|| self.order.first().copied())
    }

    pub fn registered_default_kind() -> Result<Option<PaneKind>, PaneRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .default_kind())
    }

    pub fn all_descriptors(&self) -> Vec<Arc<dyn PaneDescriptor>> {
        self.order
            .iter()
            .filter_map(|kind| self.descriptors.get(kind).cloned())
            .collect()
    }

    pub fn registered_descriptors() -> Result<Vec<Arc<dyn PaneDescriptor>>, PaneRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .all_descriptors())
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::*;

    struct TestDescriptor(PaneKind);

    impl PaneDescriptor for TestDescriptor {
        fn kind(&self) -> PaneKind {
            self.0
        }

        fn display_name(&self) -> SharedString {
            "Test".into()
        }

        fn create_pane(&self) -> Box<dyn PaneView> {
            panic!("factory is not needed by this test")
        }
    }

    #[test]
    fn duplicate_kinds_are_rejected_without_changing_order() {
        let mut registry = PaneRegistry::new();
        let kind = PaneKind::new("test.pane");
        registry
            .register(Arc::new(TestDescriptor(kind)), true)
            .unwrap();

        assert_eq!(
            registry.register(Arc::new(TestDescriptor(kind)), false),
            Err(PaneRegistryError::DuplicateKind(kind))
        );
        assert_eq!(registry.default_kind(), Some(kind));
        assert_eq!(registry.all_descriptors().len(), 1);
    }
}
