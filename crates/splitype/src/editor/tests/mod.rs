//! Editor integration-style unit tests, grouped by subsystem.
//!
//! Each topic module below covers one area of the editor; run a single
//! group with cargo test editor::tests::<module> (e.g.
//! cargo test editor::tests::table_ops). Shared helpers live here.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::TestAppContext;

use crate::editor::engine::controller::Editor;
use i18n::I18nManager;
use theme::ThemeManager;

mod about;
mod drop;
mod editing;
mod footnote_parsing;
mod footnotes;
mod geometry;
mod image_handles;
mod multi_panel;
mod pane_mode;
mod projection_styles;
mod save_export;
mod table_ops;
mod undo;
mod search;
mod tab_lifecycle;
mod window_flows;

fn init_editor_test_app(cx: &mut TestAppContext) {
    cx.update(|cx| {
        I18nManager::init(cx);
        ThemeManager::init(cx);
        crate::editor::commands::keybindings::init(cx);

    });
}

fn temp_markdown_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "splitype-{test_name}-{}-{nanos}.md",
        std::process::id()
    ))
}

fn temp_export_path(test_name: &str, extension: &str) -> PathBuf {
    let mut path = temp_markdown_path(test_name);
    path.set_extension(extension);
    path
}

fn redraw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}

/// Switch the root area's pane to the WYSIWYG editing pane.
/// New sessions start with a SourceCode pane, so without this the
/// document's blocks are never mounted and keyboard simulation never
/// reaches them.
fn ensure_wysiwyg_editing_panel(editor: &gpui::Entity<Editor>, cx: &mut gpui::App) {
    editor.update(cx, |editor, _cx| {
        let mut ids = Vec::new();
        editor.session_mut().root.tree.leaf_ids(&mut ids);
        for id in ids {
            editor
                .session_mut()
                .root
                .tree
                .set_leaf_kind(id, crate::editor::engine::session::EditorPaneKind::Wysiwyg);
        }
    });
}

/// Focus a specific block so keyboard simulation (simulate_input /
/// simulate_keystrokes) lands in the editor. The editor no longer
/// auto-focuses on window creation, key events dispatch along the
/// focused path, and the WYSIWYG panel must be mounted for the block to
/// register its input handler.
fn focus_block(
    editor: &gpui::Entity<Editor>,
    block: &gpui::Entity<editor_wysiwyg::document::block::Block>,
    cx: &mut gpui::VisualTestContext,
) {
    cx.cx
        .update(|app| ensure_wysiwyg_editing_panel(editor, app));
    editor.update(cx, |editor, _cx| {
        editor.focus_block(block.entity_id());
    });
    cx.update(|window, cx| {
        block.update(cx, |block, cx| {
            block.focus_handle.focus(window, cx);
        });
    });
    redraw(cx);
}

/// Focus the first block of the document via [ocus_block].
fn focus_first_block(editor: &gpui::Entity<Editor>, cx: &mut gpui::VisualTestContext) {
    let first = editor
        .update(cx, |editor, _cx| {
            editor
                .doc()
                .blocks()
                .first()
                .map(|entries| entries.entity.clone())
        })
        .expect("document should have a block");
    focus_block(editor, &first, cx);
}
