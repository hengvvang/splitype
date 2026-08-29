//! Workspace Integration Tests for Splitype.
//!
//! Validates end-to-end interactions across:
//! - `splitype_core` (IDs & Primitives)
//! - `splitype_model` (Markdown AST & SumTree)
//! - `splitype_splitter` (Multi-pane Tiled Splitter Engine)
//! - `splitype_infra` (Configuration, Themes, and I18n)
//! - `splitype_render` (Syntax Highlighting & HTML Export)

use splitype::core::{BlockId, DocumentId};
use splitype::infra::config::settings::AppSettings;
use splitype::infra::theme::Theme;
use splitype::model::parse::parser::parse_wysiwyg_document;
use splitype::model::parse::BlockKind;
use splitype::render::export::html::render_html;
use splitype::splitter::{SplitAxis, SplitterRoot};

#[test]
fn test_end_to_end_document_lifecycle() {
    // 1. Generate IDs
    let doc_id = DocumentId::new();
    let block_id = BlockId::next();
    assert_ne!(doc_id.to_string(), "");
    assert!(block_id.raw() > 0);

    // 2. Parse Markdown into AST (WYSIWYG 1:1 line mapping)
    let markdown = "# Splitype Architecture\n\n- Tiled Layout\n- Block Editor\n- GPUI Acceleration\n\n```rust\nfn main() {}\n```";
    let blocks = parse_wysiwyg_document(markdown);
    assert_eq!(blocks.len(), 7);
    assert_eq!(blocks[0].kind, BlockKind::Heading { level: 1 });
    assert!(matches!(blocks[2].kind, BlockKind::BulletListItem));
    assert!(matches!(blocks[6].kind, BlockKind::CodeBlock { .. }));

    // 3. Initialize Theme and Settings
    let settings = AppSettings::default();
    let theme = Theme::default_theme();
    assert_eq!(settings.typography.font_size, 16);
    assert_eq!(theme.name, "Dark");

    // 4. Export to HTML
    let html = render_html(markdown, &theme, "Integration Test Doc");
    assert!(html.contains("<h1>Splitype Architecture</h1>"));
    assert!(html.contains("<!doctype html>"));

    // 5. Initialize Splitter Engine
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Pane {
        Doc,
    }

    let mut splitter = SplitterRoot::single_leaf(1, Pane::Doc);
    let first = 1;
    let second = splitter.split_leaf(first, SplitAxis::Horizontal, 0.5).unwrap();
    assert_eq!(splitter.tree.count_leaves(), 2);
    assert_ne!(first, second);
}
