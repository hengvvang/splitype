//! Document command actions — the GPUI action protocol for editor commands.

use gpui::*;

actions!(
    splitype,
    [
        Undo,
        Redo,
        PageUp,
        PageDown,
        JumpToTop,
        JumpToBottom,
        SaveDocument,
        SaveDocumentAs,
        ExportHtml,
        ExportPdf,
        TogglePaneKind,
        ToggleMaximizePane,
        ToggleSearch,
        ToggleReplace,
        FindNext,
        FindPrevious,
        ReplaceCurrent,
        ReplaceAll,
    ]
);

