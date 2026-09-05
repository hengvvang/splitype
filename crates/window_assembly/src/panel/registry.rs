//! Global panel plugin registry.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, LazyLock, Mutex};

use gpui::App;

use platform_contracts::{PanelDescriptor, PanelId, PanelKind, PanelView};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelRegistryError {
    DuplicateKind(PanelKind),
    Poisoned,
}

impl fmt::Display for PanelRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKind(kind) => {
                write!(formatter, "panel kind '{kind}' is already registered")
            }
            Self::Poisoned => formatter.write_str("panel registry lock is poisoned"),
        }
    }
}

impl std::error::Error for PanelRegistryError {}

/// Thread-safe registry holding all available PanelDescriptors.
#[derive(Default)]
pub struct PanelRegistry {
    descriptors: HashMap<PanelKind, Arc<dyn PanelDescriptor>>,
    order: Vec<PanelKind>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn global() -> &'static Mutex<Self> {
        static REGISTRY: LazyLock<Mutex<PanelRegistry>> =
            LazyLock::new(|| Mutex::new(PanelRegistry::new()));
        &REGISTRY
    }

    pub fn register(
        &mut self,
        descriptor: Arc<dyn PanelDescriptor>,
    ) -> Result<(), PanelRegistryError> {
        let kind = descriptor.kind();
        if self.descriptors.contains_key(&kind) {
            return Err(PanelRegistryError::DuplicateKind(kind));
        }

        self.order.push(kind.clone());
        self.descriptors.insert(kind, descriptor);
        Ok(())
    }

    pub fn register_global(descriptor: Arc<dyn PanelDescriptor>) -> Result<(), PanelRegistryError> {
        Self::global()
            .lock()
            .map_err(|_| PanelRegistryError::Poisoned)?
            .register(descriptor)
    }

    pub fn get(&self, kind: PanelKind) -> Option<Arc<dyn PanelDescriptor>> {
        self.descriptors.get(&kind).cloned()
    }

    pub fn registered(
        kind: PanelKind,
    ) -> Result<Option<Arc<dyn PanelDescriptor>>, PanelRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PanelRegistryError::Poisoned)?
            .get(kind))
    }

    pub fn create_registered_panel(
        kind: PanelKind,
        panel_id: PanelId,
        cx: &mut App,
    ) -> Result<Option<Box<dyn PanelView>>, PanelRegistryError> {
        // Never execute plugin code while holding the registry mutex. A panel
        // factory may legitimately query another descriptor during creation.
        let descriptor = Self::registered(kind)?;
        Ok(descriptor.map(|descriptor| descriptor.create_panel(panel_id, cx)))
    }

    pub fn restore_registered_panel(
        kind: PanelKind,
        panel_id: PanelId,
        state: Box<dyn std::any::Any>,
        cx: &mut App,
    ) -> Result<Option<Box<dyn PanelView>>, PanelRegistryError> {
        let descriptor = Self::registered(kind)?;
        Ok(descriptor.and_then(|descriptor| descriptor.restore_panel(panel_id, state, cx)))
    }

    pub fn all_descriptors(&self) -> Vec<Arc<dyn PanelDescriptor>> {
        self.order
            .iter()
            .filter_map(|kind| self.descriptors.get(kind).cloned())
            .collect()
    }

    pub fn registered_descriptors() -> Result<Vec<Arc<dyn PanelDescriptor>>, PanelRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PanelRegistryError::Poisoned)?
            .all_descriptors())
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::*;

    struct TestDescriptor(PanelKind);

    impl PanelDescriptor for TestDescriptor {
        fn kind(&self) -> PanelKind {
            self.0.clone()
        }

        fn display_name(&self) -> SharedString {
            "Test".into()
        }

        fn create_panel(&self, _panel_id: PanelId, _cx: &mut App) -> Box<dyn PanelView> {
            panic!("factory is not needed by this test")
        }
    }

    #[test]
    fn duplicate_kinds_are_rejected_without_changing_order() {
        let mut registry = PanelRegistry::new();
        let kind = PanelKind::new("test.panel");
        registry
            .register(Arc::new(TestDescriptor(kind.clone())))
            .unwrap();

        assert_eq!(
            registry.register(Arc::new(TestDescriptor(kind.clone()))),
            Err(PanelRegistryError::DuplicateKind(kind.clone()))
        );
        assert_eq!(registry.all_descriptors().len(), 1);
    }
}
