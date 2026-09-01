//! Keybinding installation — turns the command registry's manifest-declared
//! shortcuts into gpui key bindings.
//!
//! The command registry owns the default shortcuts and key contexts; this
//! module resolves every contribution through the composition-root binding
//! table, applies user overrides and conflict resolution, and installs the
//! resulting bindings.

use std::collections::BTreeMap;

use gpui::*;

use crate::commands::binding_for;
use config::keybindings::normalize_shortcut_keys;

/// Platform policy for shortcuts declared by manifests: OS-level keys are
/// declared once and filtered here, where platform glue lives.
fn platform_allows(command_id: &str, keystroke: &str) -> bool {
    match (command_id, keystroke) {
        // cmd-q is the system quit shortcut; Windows/Linux use Alt+F4
        // (handled by the OS, no binding).
        ("splitype.core.quit", _) => cfg!(target_os = "macos"),
        // cmd-w closes the current window on macOS; other platforms use
        // ctrl-q. Both are declared and filtered per platform.
        ("splitype.core.close-window", "cmd-w") => cfg!(target_os = "macos"),
        ("splitype.core.close-window", "ctrl-q") => !cfg!(target_os = "macos"),
        _ => true,
    }
}

/// Computes the effective shortcut list for one command: user overrides win,
/// falling back to the manifest-declared defaults.
fn effective_keys(
    command_id: &str,
    defaults: &[String],
    overrides: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    overrides
        .get(command_id)
        .and_then(|keys| normalize_shortcut_keys(keys))
        .unwrap_or_else(|| defaults.to_vec())
}

/// Resolves every contributed command into its gpui key bindings, honoring
/// user overrides and dropping overrides that conflict inside one context.
pub(crate) fn resolved_keybindings(overrides: &BTreeMap<String, Vec<String>>) -> Vec<KeyBinding> {
    let contributions =
        platform_contracts::CommandRegistry::registered_commands().unwrap_or_default();

    // One pass of context-scoped conflict resolution: when a user override
    // collides with an effective shortcut of another command in the same
    // context, the override falls back to the default.
    let mut effective: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for contribution in &contributions {
        let defaults: Vec<String> = contribution
            .shortcuts
            .iter()
            .map(|s| s.to_string())
            .collect();
        effective.insert(
            contribution.id.as_str(),
            effective_keys(contribution.id.as_str(), &defaults, overrides),
        );
    }
    for left in &contributions {
        for right in &contributions {
            if left.id == right.id {
                continue;
            }
            if left.context != right.context {
                continue;
            }
            let left_keys = effective.get(left.id.as_str()).cloned().unwrap_or_default();
            let right_keys = effective
                .get(right.id.as_str())
                .cloned()
                .unwrap_or_default();
            if left_keys.iter().any(|key| right_keys.contains(key)) {
                for id in [left.id.as_str(), right.id.as_str()] {
                    if overrides.contains_key(id) {
                        let defaults: Vec<String> = contributions
                            .iter()
                            .find(|c| c.id.as_str() == id)
                            .map(|c| c.shortcuts.iter().map(|s| s.to_string()).collect())
                            .unwrap_or_default();
                        effective.insert(id, defaults);
                    }
                }
            }
        }
    }

    let mut bindings = Vec::new();
    for contribution in &contributions {
        if contribution.shortcuts.is_empty() {
            continue;
        }
        let Some(binding) = binding_for_contribution(contribution) else {
            continue;
        };
        let keys = effective
            .get(contribution.id.as_str())
            .cloned()
            .unwrap_or_default();
        for key in keys {
            if !platform_allows(contribution.id.as_str(), &key) {
                continue;
            }
            let context_predicate = contribution.context.as_deref().map(|context| {
                KeyBindingContextPredicate::parse(context)
                    .expect("manifest keybinding contexts must parse")
                    .into()
            });
            bindings.push(
                KeyBinding::load(
                    &key,
                    (binding.make_action)(),
                    context_predicate,
                    false,
                    None,
                    &DummyKeyboardMapper,
                )
                .expect("manifest shortcuts must parse as keystrokes"),
            );
        }
    }
    bindings
}

fn binding_for_contribution(
    contribution: &platform_contracts::CommandContribution,
) -> Option<crate::commands::CommandBinding> {
    let (plugin, local) = contribution.id.as_str().rsplit_once('.')?;
    binding_for(plugin, local)
}

pub(crate) fn install_keybindings(cx: &mut App, config: &BTreeMap<String, Vec<String>>) {
    cx.bind_keys(resolved_keybindings(config));
}

/// Test-only: registers default key bindings for the block editor.
pub(crate) fn init_with_keybindings(cx: &mut App, config: &BTreeMap<String, Vec<String>>) {
    install_keybindings(cx, config);
}
