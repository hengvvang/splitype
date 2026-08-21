//! Block-editing action definitions — the block-editing action protocol.
//!
//! Actions are scoped to the `"BlockEditor"` key context on each block. Window
//! and menu command actions live in `crate::app::actions`; document
//! commands live in `crate::editor::actions`; the keybinding
//! configuration table lives in `crate::editor::keybindings`.
//! Handlers for these actions live in [`super::block_actions`].

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
        DismissTransientUi,
    ]
);
