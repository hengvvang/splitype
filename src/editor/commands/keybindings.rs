//! Keybinding installation — turns the shortcut configuration into gpui
//! key bindings for the block editor and window commands.
//!
//! The shortcut schema (definition table, defaults, normalization) lives in
//! `crate::infra::config::keybindings`; this module maps `ShortcutCommand`s
//! to the concrete gpui action types and installs them.

use std::collections::BTreeMap;

use gpui::*;

use crate::app::actions::{
    CloseWindow, NewWindow, OpenFile, QuitApplication, ToggleExplorer, ToggleMaximizeArea,
};
use crate::editor::actions::{
    FindNext, FindPrevious, SaveDocument, SaveDocumentAs, ToggleMaximizePane, TogglePaneKind,
    ToggleReplace, ToggleSearch,
};
use crate::editor::input::actions::{
    BlockDown, BlockUp, BoldSelection, CodeSelection, Copy, Cut, Delete, DeleteBackward,
    DismissTransientUi, End, ExitCodeBlock, FocusNext, FocusPrevious, Home, IndentBlock,
    ItalicSelection, JumpToBottom, JumpToTop, MoveLeft, MoveRight, Newline, OutdentBlock, PageDown,
    PageUp, Paste, Redo, SelectAll, SelectEnd, SelectHome, SelectLeft, SelectRight,
    StrikethroughSelection, UnderlineSelection, Undo, WordDeleteBackward, WordDeleteForward,
    WordMoveLeft, WordMoveRight, WordSelectLeft, WordSelectRight,
};
use crate::infra::config::keybindings::{
    SHORTCUT_DEFINITIONS, ShortcutCommand, default_keys, normalize_shortcut_config,
};

fn key_binding_for(
    command: ShortcutCommand,
    key: &str,
    context: Option<&'static str>,
) -> KeyBinding {
    match command {
        ShortcutCommand::Newline => KeyBinding::new(key, Newline, context),
        ShortcutCommand::DeleteBackward => KeyBinding::new(key, DeleteBackward, context),
        ShortcutCommand::Delete => KeyBinding::new(key, Delete, context),
        ShortcutCommand::WordDeleteBackward => KeyBinding::new(key, WordDeleteBackward, context),
        ShortcutCommand::WordDeleteForward => KeyBinding::new(key, WordDeleteForward, context),
        ShortcutCommand::FocusPrevious => KeyBinding::new(key, FocusPrevious, context),
        ShortcutCommand::FocusNext => KeyBinding::new(key, FocusNext, context),
        ShortcutCommand::MoveLeft => KeyBinding::new(key, MoveLeft, context),
        ShortcutCommand::MoveRight => KeyBinding::new(key, MoveRight, context),
        ShortcutCommand::WordMoveLeft => KeyBinding::new(key, WordMoveLeft, context),
        ShortcutCommand::WordMoveRight => KeyBinding::new(key, WordMoveRight, context),
        ShortcutCommand::Home => KeyBinding::new(key, Home, context),
        ShortcutCommand::End => KeyBinding::new(key, End, context),
        ShortcutCommand::BlockUp => KeyBinding::new(key, BlockUp, context),
        ShortcutCommand::BlockDown => KeyBinding::new(key, BlockDown, context),
        ShortcutCommand::PageUp => KeyBinding::new(key, PageUp, context),
        ShortcutCommand::PageDown => KeyBinding::new(key, PageDown, context),
        ShortcutCommand::JumpToTop => KeyBinding::new(key, JumpToTop, context),
        ShortcutCommand::JumpToBottom => KeyBinding::new(key, JumpToBottom, context),
        ShortcutCommand::SelectLeft => KeyBinding::new(key, SelectLeft, context),
        ShortcutCommand::SelectRight => KeyBinding::new(key, SelectRight, context),
        ShortcutCommand::WordSelectLeft => KeyBinding::new(key, WordSelectLeft, context),
        ShortcutCommand::WordSelectRight => KeyBinding::new(key, WordSelectRight, context),
        ShortcutCommand::SelectHome => KeyBinding::new(key, SelectHome, context),
        ShortcutCommand::SelectEnd => KeyBinding::new(key, SelectEnd, context),
        ShortcutCommand::SelectAll => KeyBinding::new(key, SelectAll, context),
        ShortcutCommand::Copy => KeyBinding::new(key, Copy, context),
        ShortcutCommand::Cut => KeyBinding::new(key, Cut, context),
        ShortcutCommand::Paste => KeyBinding::new(key, Paste, context),
        ShortcutCommand::Undo => KeyBinding::new(key, Undo, context),
        ShortcutCommand::Redo => KeyBinding::new(key, Redo, context),
        ShortcutCommand::BoldSelection => KeyBinding::new(key, BoldSelection, context),
        ShortcutCommand::ItalicSelection => KeyBinding::new(key, ItalicSelection, context),
        ShortcutCommand::UnderlineSelection => KeyBinding::new(key, UnderlineSelection, context),
        ShortcutCommand::CodeSelection => KeyBinding::new(key, CodeSelection, context),
        ShortcutCommand::StrikethroughSelection => {
            KeyBinding::new(key, StrikethroughSelection, context)
        }
        ShortcutCommand::IndentBlock => KeyBinding::new(key, IndentBlock, context),
        ShortcutCommand::OutdentBlock => KeyBinding::new(key, OutdentBlock, context),
        ShortcutCommand::ExitCodeBlock => KeyBinding::new(key, ExitCodeBlock, context),
        ShortcutCommand::SaveDocument => KeyBinding::new(key, SaveDocument, context),
        ShortcutCommand::SaveDocumentAs => KeyBinding::new(key, SaveDocumentAs, context),
        ShortcutCommand::NewWindow => KeyBinding::new(key, NewWindow, context),
        ShortcutCommand::OpenFile => KeyBinding::new(key, OpenFile, context),
        ShortcutCommand::QuitApplication => KeyBinding::new(key, QuitApplication, context),
        ShortcutCommand::CloseWindow => KeyBinding::new(key, CloseWindow, context),
        ShortcutCommand::DismissTransientUi => KeyBinding::new(key, DismissTransientUi, context),
        ShortcutCommand::TogglePaneKind => KeyBinding::new(key, TogglePaneKind, context),
        ShortcutCommand::ToggleExplorer => KeyBinding::new(key, ToggleExplorer, context),
        ShortcutCommand::ToggleMaximizeArea => KeyBinding::new(key, ToggleMaximizeArea, context),
        ShortcutCommand::ToggleMaximizePane => KeyBinding::new(key, ToggleMaximizePane, context),
        ShortcutCommand::ToggleSearch => KeyBinding::new(key, ToggleSearch, context),
        ShortcutCommand::ToggleReplace => KeyBinding::new(key, ToggleReplace, context),
        ShortcutCommand::FindNext => KeyBinding::new(key, FindNext, context),
        ShortcutCommand::FindPrevious => KeyBinding::new(key, FindPrevious, context),
    }
}

pub(crate) fn resolved_keybindings(config: &BTreeMap<String, Vec<String>>) -> Vec<KeyBinding> {
    let normalized = normalize_shortcut_config(config);
    let mut bindings = Vec::new();
    for definition in SHORTCUT_DEFINITIONS {
        let keys = normalized
            .get(definition.id)
            .cloned()
            .unwrap_or_else(|| default_keys(*definition));
        bindings.extend(
            keys.iter()
                .map(|key| key_binding_for(definition.command, key, definition.context)),
        );
    }
    bindings
}

pub(crate) fn install_keybindings(cx: &mut App, config: &BTreeMap<String, Vec<String>>) {
    cx.bind_keys(resolved_keybindings(config));
}

/// Test-only: registers default key bindings for the block editor.
#[cfg(test)]
pub fn init(cx: &mut App) {
    install_keybindings(cx, &BTreeMap::new());
}

pub(crate) fn init_with_keybindings(cx: &mut App, config: &BTreeMap<String, Vec<String>>) {
    install_keybindings(cx, config);
}
