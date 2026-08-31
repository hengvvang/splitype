//! Command registry — the discovery record of every contributed command.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use thiserror::Error;

use crate::command::CommandId;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CommandRegistryError {
    #[error("command '{0}' is already registered")]
    DuplicateCommand(CommandId),
    #[error("command registry lock is poisoned")]
    Poisoned,
}

/// A plugin's command contribution: where it appears and its default
/// shortcut. The concrete action and localized label are provided by the
/// composition root's command binding table, keyed by the same id.
#[derive(Clone, Debug)]
pub struct CommandContribution {
    pub id: CommandId,
    /// Menu skeleton location, e.g. `file` or `file.export`. `None` for
    /// keybinding-only commands.
    pub menu: Option<Arc<str>>,
    /// Default shortcut, as display/definition metadata.
    pub shortcut: Option<Arc<str>>,
}

/// Registry of command contributions, keyed by command id.
#[derive(Default)]
pub struct CommandRegistry {
    order: Vec<CommandId>,
    commands: HashMap<CommandId, Arc<CommandContribution>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn global() -> &'static Mutex<Self> {
        static REGISTRY: LazyLock<Mutex<CommandRegistry>> =
            LazyLock::new(|| Mutex::new(CommandRegistry::new()));
        &REGISTRY
    }

    pub fn register(
        &mut self,
        contribution: CommandContribution,
    ) -> Result<(), CommandRegistryError> {
        let id = contribution.id.clone();
        if self.commands.contains_key(&id) {
            return Err(CommandRegistryError::DuplicateCommand(id));
        }
        self.order.push(id.clone());
        self.commands.insert(id, Arc::new(contribution));
        Ok(())
    }

    pub fn register_global(contribution: CommandContribution) -> Result<(), CommandRegistryError> {
        Self::global()
            .lock()
            .map_err(|_| CommandRegistryError::Poisoned)?
            .register(contribution)
    }

    pub fn get(&self, id: &CommandId) -> Option<Arc<CommandContribution>> {
        self.commands.get(id).cloned()
    }

    pub fn registered(
        id: CommandId,
    ) -> Result<Option<Arc<CommandContribution>>, CommandRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| CommandRegistryError::Poisoned)?
            .get(&id))
    }

    pub fn all(&self) -> Vec<Arc<CommandContribution>> {
        self.order
            .iter()
            .filter_map(|id| self.commands.get(id).cloned())
            .collect()
    }

    pub fn registered_commands() -> Result<Vec<Arc<CommandContribution>>, CommandRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| CommandRegistryError::Poisoned)?
            .all())
    }

    /// Contributions located at `menu` in declaration order.
    pub fn in_menu(&self, menu: &str) -> Vec<Arc<CommandContribution>> {
        self.order
            .iter()
            .filter_map(|id| self.commands.get(id))
            .filter(|command| command.menu.as_deref() == Some(menu))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(id: &'static str) -> CommandContribution {
        CommandContribution {
            id: CommandId::from_static(id),
            menu: None,
            shortcut: None,
        }
    }

    #[test]
    fn duplicate_command_ids_are_rejected() {
        let mut registry = CommandRegistry::new();
        registry.register(command("splitype.editor.save")).unwrap();
        assert_eq!(
            registry.register(command("splitype.editor.save")),
            Err(CommandRegistryError::DuplicateCommand(
                CommandId::from_static("splitype.editor.save")
            ))
        );
        assert_eq!(registry.all().len(), 1);
    }
}
