//! Keybinding schema — the shortcut definition table and configuration
//! normalization.
//!
//! Owns the canonical list of shortcut commands, their default keys, and
//! the normalization/conflict logic that settings read. Pure data plus
//! gpui's `Keystroke` parser; no editor imports, so `infra` never depends
//! on the application layers. The runtime key-binding installation lives
//! in `crate::editor::keybindings`.

use std::collections::{BTreeMap, BTreeSet};

use gpui::Keystroke;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutCommand {
    Newline,
    DeleteBackward,
    Delete,
    WordDeleteBackward,
    WordDeleteForward,
    FocusPrevious,
    FocusNext,
    MoveLeft,
    MoveRight,
    WordMoveLeft,
    WordMoveRight,
    Home,
    End,
    BlockUp,
    BlockDown,
    PageUp,
    PageDown,
    JumpToTop,
    JumpToBottom,
    SelectLeft,
    SelectRight,
    WordSelectLeft,
    WordSelectRight,
    SelectHome,
    SelectEnd,
    SelectAll,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
    BoldSelection,
    ItalicSelection,
    UnderlineSelection,
    CodeSelection,
    StrikethroughSelection,
    IndentBlock,
    OutdentBlock,
    ExitCodeBlock,
    SaveDocument,
    SaveDocumentAs,
    NewWindow,
    OpenFile,
    QuitApplication,
    CloseWindow,
    DismissTransientUi,
    TogglePaneKind,
    ToggleExplorer,
    ToggleMaximizeArea,
    ToggleMaximizePane,
}

#[derive(Clone, Copy, Debug)]
pub struct ShortcutDefinition {
    pub command: ShortcutCommand,
    pub id: &'static str,
    pub default_keys: &'static [&'static str],
    pub context: Option<&'static str>,
}

const BLOCK_CONTEXT: Option<&str> = Some("BlockEditor");
const SELECT_ALL_ID: &str = "select_all";

// On macOS cmd-q is the system quit shortcut; Windows/Linux use Alt+F4 (OS-handled).
#[cfg(target_os = "macos")]
const QUIT_APPLICATION_DEFAULT_KEYS: &[&str] = &["cmd-q"];
#[cfg(not(target_os = "macos"))]
const QUIT_APPLICATION_DEFAULT_KEYS: &[&str] = &[];

// On macOS cmd-w closes the current window; no app-level binding needed on other platforms.
#[cfg(target_os = "macos")]
const CLOSE_WINDOW_DEFAULT_KEYS: &[&str] = &["cmd-w"];
#[cfg(not(target_os = "macos"))]
const CLOSE_WINDOW_DEFAULT_KEYS: &[&str] = &["ctrl-q"];

pub const SHORTCUT_DEFINITIONS: &[ShortcutDefinition] = &[
    ShortcutDefinition {
        command: ShortcutCommand::Newline,
        id: "newline",
        default_keys: &["enter"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::DeleteBackward,
        id: "delete_back",
        default_keys: &["backspace"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::Delete,
        id: "delete",
        default_keys: &["delete"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::WordDeleteBackward,
        id: "word_delete_back",
        default_keys: &["ctrl-backspace", "alt-backspace"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::WordDeleteForward,
        id: "word_delete_forward",
        default_keys: &["ctrl-delete", "alt-delete"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::FocusPrevious,
        id: "focus_previous",
        default_keys: &["up"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::FocusNext,
        id: "focus_next",
        default_keys: &["down"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::MoveLeft,
        id: "move_left",
        default_keys: &["left"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::MoveRight,
        id: "move_right",
        default_keys: &["right"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::WordMoveLeft,
        id: "word_move_left",
        default_keys: &["ctrl-left", "alt-left"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::WordMoveRight,
        id: "word_move_right",
        default_keys: &["ctrl-right", "alt-right"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::Home,
        id: "home",
        default_keys: &["home"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::End,
        id: "end",
        default_keys: &["end"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::BlockUp,
        id: "block_up",
        default_keys: &["ctrl-up", "alt-up"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::BlockDown,
        id: "block_down",
        default_keys: &["ctrl-down", "alt-down"],
        context: BLOCK_CONTEXT,
    },
    // Page scroll and document jumps operate on the editor viewport rather than
    // a single block, so they use global bindings (no context) and stay active
    // in both Rendered and Source mode.
    ShortcutDefinition {
        command: ShortcutCommand::PageUp,
        id: "page_up",
        default_keys: &["pageup"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::PageDown,
        id: "page_down",
        default_keys: &["pagedown"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::JumpToTop,
        id: "jump_to_top",
        default_keys: &["ctrl-home", "cmd-up"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::JumpToBottom,
        id: "jump_to_bottom",
        default_keys: &["ctrl-end", "cmd-down"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::SelectLeft,
        id: "select_left",
        default_keys: &["shift-left"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::SelectRight,
        id: "select_right",
        default_keys: &["shift-right"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::WordSelectLeft,
        id: "word_select_left",
        default_keys: &["ctrl-shift-left", "alt-shift-left"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::WordSelectRight,
        id: "word_select_right",
        default_keys: &["ctrl-shift-right", "alt-shift-right"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::SelectHome,
        id: "select_home",
        default_keys: &["shift-home"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::SelectEnd,
        id: "select_end",
        default_keys: &["shift-end"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::SelectAll,
        id: SELECT_ALL_ID,
        default_keys: &["cmd-a", "ctrl-a"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::Copy,
        id: "copy",
        default_keys: &["cmd-c", "ctrl-c"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::Cut,
        id: "cut",
        default_keys: &["cmd-x", "ctrl-x"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::Paste,
        id: "paste",
        default_keys: &["cmd-v", "ctrl-v"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::Undo,
        id: "undo",
        default_keys: &["cmd-z", "ctrl-z"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::Redo,
        id: "redo",
        default_keys: &["cmd-shift-z", "ctrl-y"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::BoldSelection,
        id: "bold_selection",
        default_keys: &["cmd-b", "ctrl-b"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::ItalicSelection,
        id: "italic_selection",
        default_keys: &["cmd-i", "ctrl-i"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::UnderlineSelection,
        id: "underline_selection",
        default_keys: &["cmd-u", "ctrl-u"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::CodeSelection,
        id: "code_selection",
        default_keys: &["cmd-`", "ctrl-`"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::StrikethroughSelection,
        id: "strikethrough_selection",
        default_keys: &["cmd-shift-x", "ctrl-shift-x", "alt-shift-5"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::IndentBlock,
        id: "indent_block",
        default_keys: &["tab"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::OutdentBlock,
        id: "outdent_block",
        default_keys: &["shift-tab"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::ExitCodeBlock,
        id: "exit_code_block",
        default_keys: &["cmd-enter", "ctrl-enter"],
        context: BLOCK_CONTEXT,
    },
    ShortcutDefinition {
        command: ShortcutCommand::SaveDocument,
        id: "save_document",
        default_keys: &["cmd-s", "ctrl-s"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::SaveDocumentAs,
        id: "save_document_as",
        default_keys: &["cmd-shift-s", "ctrl-shift-s"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::NewWindow,
        id: "new_window",
        default_keys: &["cmd-n", "ctrl-n"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::OpenFile,
        id: "open_file",
        default_keys: &["cmd-o", "ctrl-o"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::QuitApplication,
        id: "quit_application",
        default_keys: QUIT_APPLICATION_DEFAULT_KEYS,
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::CloseWindow,
        id: "close_window",
        default_keys: CLOSE_WINDOW_DEFAULT_KEYS,
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::DismissTransientUi,
        id: "dismiss_transient_ui",
        default_keys: &["escape"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::TogglePaneKind,
        id: "toggle_pane_kind",
        default_keys: &["ctrl-tab", "cmd-tab"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::ToggleExplorer,
        id: "toggle_explorer",
        default_keys: &["ctrl-w"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::ToggleMaximizeArea,
        id: "toggle_maximize_area",
        default_keys: &["ctrl-space", "cmd-space"],
        context: None,
    },
    ShortcutDefinition {
        command: ShortcutCommand::ToggleMaximizePane,
        id: "toggle_maximize_pane",
        default_keys: &["ctrl-shift-m", "cmd-shift-m"],
        context: None,
    },
];

pub fn normalize_shortcut_keys(keys: &[String]) -> Option<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for key in keys {
        let parsed = Keystroke::parse(key.trim()).ok()?;
        if parsed.is_ime_in_progress() {
            return None;
        }
        let key = parsed.unparse();
        if seen.insert(key.clone()) {
            normalized.push(key);
        }
    }
    (!normalized.is_empty()).then_some(normalized)
}

pub fn default_keys(definition: ShortcutDefinition) -> Vec<String> {
    definition
        .default_keys
        .iter()
        .map(|key| key.to_string())
        .collect()
}

/// Reads a user shortcut override for the definition id.
fn configured_shortcut_keys(
    definition: ShortcutDefinition,
    config: &BTreeMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    config
        .get(definition.id)
        .and_then(|keys| normalize_shortcut_keys(keys))
}

fn shortcuts_conflict(
    left: ShortcutDefinition,
    left_keys: &[String],
    right: ShortcutDefinition,
    right_keys: &[String],
) -> bool {
    left.context == right.context && left_keys.iter().any(|key| right_keys.contains(key))
}

pub fn normalize_shortcut_config(
    config: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    let mut effective: BTreeMap<&'static str, (bool, Vec<String>)> = BTreeMap::new();
    for definition in SHORTCUT_DEFINITIONS {
        let custom = configured_shortcut_keys(*definition, config);
        effective.insert(
            definition.id,
            match custom {
                Some(keys) if keys != default_keys(*definition) => (true, keys),
                _ => (false, default_keys(*definition)),
            },
        );
    }

    loop {
        let mut conflicted = BTreeSet::new();
        for (index, left) in SHORTCUT_DEFINITIONS.iter().enumerate() {
            let (left_custom, left_keys) = effective.get(left.id).expect("known shortcut");
            for right in SHORTCUT_DEFINITIONS.iter().skip(index + 1) {
                let (right_custom, right_keys) = effective.get(right.id).expect("known shortcut");
                if shortcuts_conflict(*left, left_keys, *right, right_keys) {
                    if *left_custom {
                        conflicted.insert(left.id);
                    }
                    if *right_custom {
                        conflicted.insert(right.id);
                    }
                }
            }
        }

        if conflicted.is_empty() {
            break;
        }

        for id in conflicted {
            if let Some(definition) = SHORTCUT_DEFINITIONS
                .iter()
                .find(|definition| definition.id == id)
            {
                effective.insert(definition.id, (false, default_keys(*definition)));
            }
        }
    }

    effective
        .into_iter()
        .filter_map(|(id, (custom, keys))| custom.then_some((id.to_string(), keys)))
        .collect()
}

#[cfg(test)]
pub fn resolved_shortcut_keys(
    config: &BTreeMap<String, Vec<String>>,
    command: ShortcutCommand,
) -> Vec<String> {
    let normalized = normalize_shortcut_config(config);
    let definition = SHORTCUT_DEFINITIONS
        .iter()
        .find(|definition| definition.command == command)
        .expect("known shortcut command");
    normalized
        .get(definition.id)
        .cloned()
        .unwrap_or_else(|| default_keys(*definition))
}

#[cfg(test)]
pub fn shortcut_conflict_for(
    command: ShortcutCommand,
    proposed_keys: &[String],
    config: &BTreeMap<String, Vec<String>>,
) -> Option<ShortcutDefinition> {
    let definition = SHORTCUT_DEFINITIONS
        .iter()
        .find(|definition| definition.command == command)?;
    let proposed_keys = normalize_shortcut_keys(proposed_keys)?;
    for other in SHORTCUT_DEFINITIONS
        .iter()
        .filter(|other| other.command != command)
    {
        let other_keys = resolved_shortcut_keys(config, other.command);
        if shortcuts_conflict(*definition, &proposed_keys, *other, &other_keys) {
            return Some(*other);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        ShortcutCommand, normalize_shortcut_config, resolved_shortcut_keys, shortcut_conflict_for,
    };
    use std::collections::BTreeMap;

    #[test]
    fn custom_shortcut_replaces_command_defaults() {
        let mut config = BTreeMap::new();
        config.insert("save_document".to_string(), vec!["ctrl-alt-s".to_string()]);

        assert_eq!(
            resolved_shortcut_keys(&config, ShortcutCommand::SaveDocument),
            vec!["ctrl-alt-s".to_string()]
        );
    }

    #[test]
    fn toggle_pane_kind_has_default_shortcuts() {
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::TogglePaneKind),
            vec!["ctrl-tab".to_string(), "cmd-tab".to_string()]
        );
    }

    #[test]
    fn toggle_explorer_has_default_shortcuts() {
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::ToggleExplorer),
            vec!["ctrl-w".to_string()]
        );
    }

    #[test]
    fn select_all_has_default_shortcuts() {
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::SelectAll),
            vec!["cmd-a".to_string(), "ctrl-a".to_string()]
        );
        assert!(
            shortcut_conflict_for(
                ShortcutCommand::SelectAll,
                &["cmd-a".to_string(), "ctrl-a".to_string()],
                &BTreeMap::new()
            )
            .is_none()
        );
    }

    #[test]
    fn select_all_shortcut_can_be_customized() {
        let mut config = BTreeMap::new();
        config.insert("select_all".to_string(), vec!["ctrl-shift-a".to_string()]);

        assert_eq!(
            resolved_shortcut_keys(&config, ShortcutCommand::SelectAll),
            vec!["ctrl-shift-a".to_string()]
        );
    }

    #[test]
    fn close_and_quit_defaults_are_platform_specific() {
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::CloseWindow),
                vec!["cmd-w".to_string()]
            );
            assert_eq!(
                resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::QuitApplication),
                vec!["cmd-q".to_string()]
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(
                resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::CloseWindow),
                vec!["ctrl-q".to_string()]
            );
            assert!(
                resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::QuitApplication)
                    .is_empty()
            );
        }
    }

    #[test]
    fn word_and_block_shortcuts_have_ctrl_and_alt_defaults() {
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::WordMoveLeft),
            vec!["ctrl-left".to_string(), "alt-left".to_string()]
        );
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::WordDeleteBackward),
            vec!["ctrl-backspace".to_string(), "alt-backspace".to_string()]
        );
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::BlockUp),
            vec!["ctrl-up".to_string(), "alt-up".to_string()]
        );
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::WordSelectRight),
            vec![
                "ctrl-shift-right".to_string(),
                "alt-shift-right".to_string()
            ]
        );
    }

    #[test]
    fn page_navigation_shortcuts_have_defaults() {
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::PageUp),
            vec!["pageup".to_string()]
        );
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::PageDown),
            vec!["pagedown".to_string()]
        );
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::JumpToTop),
            vec!["ctrl-home".to_string(), "cmd-up".to_string()]
        );
        assert_eq!(
            resolved_shortcut_keys(&BTreeMap::new(), ShortcutCommand::JumpToBottom),
            vec!["ctrl-end".to_string(), "cmd-down".to_string()]
        );
    }

    #[test]
    fn invalid_or_empty_shortcuts_fall_back_to_defaults() {
        let mut config = BTreeMap::new();
        config.insert("save_document".to_string(), vec!["".to_string()]);
        config.insert("open_file".to_string(), vec!["a".to_string()]);

        let normalized = normalize_shortcut_config(&config);
        assert!(!normalized.contains_key("save_document"));
        assert!(!normalized.contains_key("open_file"));
    }

    #[test]
    fn conflicting_custom_shortcut_falls_back_to_default() {
        let mut config = BTreeMap::new();
        config.insert("copy".to_string(), vec!["ctrl-x".to_string()]);

        let normalized = normalize_shortcut_config(&config);
        assert!(!normalized.contains_key("copy"));
        assert_eq!(
            resolved_shortcut_keys(&config, ShortcutCommand::Copy),
            vec!["cmd-c".to_string(), "ctrl-c".to_string()]
        );
    }

    #[test]
    fn detects_shortcut_conflicts_for_preferences_drafts() {
        let conflict = shortcut_conflict_for(
            ShortcutCommand::Copy,
            &["ctrl-x".to_string()],
            &BTreeMap::new(),
        )
        .expect("copy should conflict with cut");

        assert_eq!(conflict.id, "cut");
    }
}
