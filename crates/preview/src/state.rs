//! PreviewPane state — read-only block tree, selection and sync markers.

use std::sync::Arc;

use editor_contracts::OutlineNode;
use editor_contracts::{PaneKind, PaneOutlineHost, PaneRenderContext, PaneView};
use editor_contracts::{SearchMatch, SearchQuery};
use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window};
use theme::Theme;

use crate::node::PreviewBlock;

/// Read-only block tree shown in the preview panel.
#[derive(Default)]
pub struct PreviewState {
    pub blocks: Vec<PreviewBlock>,
    pub source_hash: u64,
    /// Document revision the preview tree was last synced at; `None` until
    /// the first build.
    pub synced_revision: Option<u64>,
}

impl PaneView for PreviewState {
    fn kind(&self) -> PaneKind {
        PaneKind::from_static(crate::builder::PANE_KIND)
    }

    fn capabilities(&self) -> editor_contracts::PaneCapabilities {
        editor_contracts::PaneCapabilities {
            editable: false,
            searchable: true,
            replaceable: false,
            outline: true,
        }
    }

    fn sync_document(&mut self, document: &editor_contracts::DocumentSnapshot, _cx: &mut App) {
        let text = document.text.as_ref();
        let revision = document.revision;
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            h.finish()
        };
        if self.source_hash == hash && !self.blocks.is_empty() {
            self.synced_revision = Some(revision);
            return;
        }
        let data = markdown_parser::parse::parse_preview_document(text);
        let mut roots = crate::node::blocks_to_preview_tree(data);
        if roots.is_empty() {
            roots.push(PreviewBlock::new(
                markdown_parser::parse::BlockData::paragraph(String::new()),
            ));
        }
        let footnote_registry = Arc::new(crate::context::build_preview_footnote_registry(&roots));
        crate::context::sync_preview_block_context(
            &mut roots,
            document.base_dir.as_deref(),
            &Default::default(),
            &footnote_registry,
        );
        self.blocks = roots;
        self.source_hash = hash;
        self.synced_revision = Some(revision);
    }

    fn document_text(&self, _cx: &App) -> Option<String> {
        // The preview is read-only: it never produces document text for the
        // host to commit.
        None
    }

    fn outline_headings(&self, _cx: &App) -> Vec<OutlineNode> {
        crate::outline::extract_preview_headings(&self.blocks)
    }

    fn navigate_to_outline(&mut self, index: usize, theme: &Theme, _cx: &mut App) -> Option<f32> {
        let headings = self.outline_headings(_cx);
        if let Some(node) = headings.get(index) {
            let font_size = theme.typography.text_size.max(14.0);
            let line_height = (font_size * theme.typography.text_line_height)
                .round()
                .max(22.0);
            let target_y =
                crate::outline::calculate_scroll_offset_for_node(self, node, line_height);
            Some(target_y)
        } else {
            None
        }
    }

    fn search_matches(&self, query: &SearchQuery, _cx: &App) -> Vec<SearchMatch> {
        let text = self
            .blocks
            .iter()
            .map(|b| b.display_text().to_string())
            .collect::<Vec<_>>()
            .join("\n\n");
        crate::search::search_in_preview(&text, query)
    }

    fn navigate_to_search_match(&mut self, match_item: &SearchMatch, cx: &mut App) -> Option<f32> {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let font_size = theme.typography.text_size.max(14.0);
        let line_height = (font_size * theme.typography.text_line_height)
            .round()
            .max(22.0);
        crate::search::calculate_scroll_offset_for_match(self, match_item, line_height)
    }

    fn set_search_highlights(
        &mut self,
        matches: &[SearchMatch],
        active_index: Option<usize>,
        _cx: &mut App,
    ) {
        crate::search::distribute_search_highlights(&mut self.blocks, matches, active_index);
    }

    fn render(&mut self, ctx: &PaneRenderContext, window: &mut Window, cx: &mut App) -> AnyElement {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let strings = config::language::I18nStrings::en_us();
        let preview_body =
            crate::render::render_preview_pane(self, ctx, &theme, &strings, window, cx);
        let headings = self.outline_headings(cx);
        let font_size = theme.typography.text_size.max(14.0);
        let line_height = (font_size * theme.typography.text_line_height)
            .round()
            .max(22.0);
        let scroll_y = -f32::from(ctx.scroll.offset().y);
        let active_index = headings
            .iter()
            .rposition(|node| {
                let node_y =
                    crate::outline::calculate_scroll_offset_for_node(self, node, line_height);
                node_y <= scroll_y + 20.0
            })
            .or(if headings.is_empty() { None } else { Some(0) });

        let outline_host: Arc<dyn editor_contracts::OutlineHost> = Arc::new(PaneOutlineHost {
            pane_id: ctx.pane_id,
            host: ctx.host.clone(),
        });
        let outline_hud = ui::render_floating_outline_hud(
            ctx.pane_id.0,
            &headings,
            active_index,
            ctx.is_outline_hovered,
            &theme,
            &outline_host,
        );
        gpui::div()
            .relative()
            .w_full()
            .h_full()
            .child(preview_body)
            .child(outline_hud)
            .into_any_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
