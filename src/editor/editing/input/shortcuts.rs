//! Text-editing action definitions — the block-editing action protocol.
//!
//! Actions are scoped to the `"BlockEditor"` key context on each block. Window
//! and menu command actions live in `crate::editor::actions`; the
//! keybinding configuration table lives in `crate::editor::keybindings`.

use gpui::*;

actions!(
    splitype,
    [
        Newline,
        DeleteBack,
        Delete,
        WordDeleteBack,
        WordDeleteForward,
        FocusPrev,
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
        IndentBlock,
        OutdentBlock,
        ExitCodeBlock,
        DismissTransientUi,
    ]
);
