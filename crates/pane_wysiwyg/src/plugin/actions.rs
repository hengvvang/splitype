//! Block-editing action definitions — the block-editing action protocol.

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
        SelectLeft,
        SelectRight,
        WordSelectLeft,
        WordSelectRight,
        SelectHome,
        SelectEnd,
        SelectAll,
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


