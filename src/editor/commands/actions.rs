//! Document command actions — the GPUI action protocol for commands that
//! target the editor's own document (save, export, view mode).
//!
//! Window / application command actions live in `crate::app::actions`;
//! text-editing actions and the keybinding configuration table live in
//! `editing::input::actions`.

use gpui::*;

actions!(
    splitype,
    [
        SaveDocument,
        SaveDocumentAs,
        ExportHtml,
        ExportPdf,
        TogglePaneKind,
        ToggleMaximizePane,
    ]
);
