//! Editor window creation and file-open routing.

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use gpui::*;

use crate::app::menus::install_menus;
use crate::app::shell::{AreaContent, Shell};
use crate::editor::controller::Editor;
use crate::infra::config::recent::record_recent_file;
use crate::splitter::DEFAULT_EDITOR_AREA_ID;
use crate::ui::custom_titlebar::splitype_window_options;

fn window_title(file_path: Option<&Path>) -> SharedString {
    if let Some(path) = file_path {
        // OsStr::to_string_lossy returns Cow<str>; calling .to_string() on
        // it always allocates a fresh String, even for the valid-UTF-8 path
        // (the common case). Borrow the Cow directly into format! — its
        // Display impl writes the borrowed bytes straight into the output
        // String, no intermediate allocation.
        format!(
            "Splitype - {}",
            path.file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_else(|| path.to_string_lossy())
        )
        .into()
    } else {
        SharedString::new("Splitype")
    }
}

/// Opens an editor window for the given Markdown content and optional path.
pub(crate) fn open_editor_window(
    cx: &mut App,
    markdown: String,
    file_path: Option<PathBuf>,
) -> WindowHandle<Shell> {
    let bounds = Bounds::centered(None, size(px(1080.), px(720.)), cx);
    let title = window_title(file_path.as_deref());
    let handle = cx
        .open_window(
            splitype_window_options(title, bounds),
            move |_window, cx| {
                let editor = cx.new(|cx| {
                    // No content and no path → welcome state with zero tabs.
                    if markdown.is_empty() && file_path.is_none() {
                        Editor::empty_in_shell(None, DEFAULT_EDITOR_AREA_ID, cx)
                    } else {
                        Editor::from_markdown_in_shell(
                            None,
                            DEFAULT_EDITOR_AREA_ID,
                            cx,
                            markdown,
                            file_path,
                        )
                    }
                });
                cx.new(move |_cx| Shell {
                    // The default layout is Explorer (left) + Editor (right);
                    // only Editor areas carry content entities.
                    areas: [(DEFAULT_EDITOR_AREA_ID, AreaContent::Editor(editor))].into(),
                })
            },
        )
        .unwrap();

    handle
        .update(cx, |shell, window, cx| {
            window.activate_window();
            let shell_weak = cx.entity().downgrade();
            if let Some(editor) = shell.primary_editor() {
                editor.update(cx, |editor, cx| {
                    editor.shell = Some(shell_weak);
                    editor.force_install_close_guard(cx, window);
                });
            }
        })
        .expect("newly opened shell window should be updateable");

    handle
}

pub(crate) fn open_file_in_new_window(cx: &mut App, path: &Path) -> anyhow::Result<()> {
    let markdown = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    open_editor_window(cx, markdown, Some(path.to_path_buf()));
    record_recent_file_and_refresh(path, cx);
    Ok(())
}

pub(crate) fn record_recent_file_and_refresh(path: &Path, cx: &mut App) {
    if let Err(err) = record_recent_file(path) {
        eprintln!("failed to update recent file history: {err}");
        return;
    }
    install_menus(cx);
    cx.refresh_windows();
}
