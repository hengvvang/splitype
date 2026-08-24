//! Single source of truth for settings shortcut definitions and metadata.

/// Descriptive metadata for a single shortcut item displayed in settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutItem {
    pub name: &'static str,
    pub description: &'static str,
    pub shortcut: &'static str,
}

/// Returns the standard document action shortcuts list for settings UI.
pub fn doc_action_shortcuts() -> &'static [ShortcutItem] {
    &[
        ShortcutItem {
            name: "Save Document",
            description: "Save active file changes to disk",
            shortcut: "Ctrl + S",
        },
        ShortcutItem {
            name: "Save Document As",
            description: "Save active document with a new name",
            shortcut: "Ctrl + Shift + S",
        },
        ShortcutItem {
            name: "New Window",
            description: "Open a new editor window instance",
            shortcut: "Ctrl + N",
        },
        ShortcutItem {
            name: "Close Window",
            description: "Close the currently focused editor window",
            shortcut: "Ctrl + W",
        },
    ]
}

/// Returns the standard interface and view control shortcuts list for settings UI.
pub fn interface_view_shortcuts() -> &'static [ShortcutItem] {
    &[
        ShortcutItem {
            name: "Toggle View Mode",
            description: "Switch between Edit, Preview, and Dual view layouts",
            shortcut: "Ctrl + M",
        },
        ShortcutItem {
            name: "Toggle Pane Maximize",
            description: "Maximize or restore the currently focused inner pane",
            shortcut: "Ctrl + Shift + M",
        },
        ShortcutItem {
            name: "Toggle ExplorerState Tree",
            description: "Show or collapse the left file navigation sidebar",
            shortcut: "Ctrl + E",
        },
        ShortcutItem {
            name: "Quit Application",
            description: "Safely exit application and save session",
            shortcut: "Ctrl + Q",
        },
    ]
}
