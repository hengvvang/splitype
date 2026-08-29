//! Block-editing action definitions — the block-editing action protocol.
//!
//! Actions are scoped to the `"BlockEditor"` key context on each block.
//! Window and menu command actions live in `crate::app::actions`; document
//! commands live in `crate::editor::commands::actions`; the keybinding
//! configuration table lives in `crate::editor::commands::keybindings`.
//! Handlers for these actions live in `crate::editor::input::block_events`.
//! The generic editing actions (`Copy`/`Cut`/`Paste`/`DismissTransientUi`)
//! live in `workspace::actions` (shared with the explorer).

use gpui::*;

actions!(
    splitype,
    [
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
    ]
);
