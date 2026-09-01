//! PreviewPane state — read-only block tree, selection and sync markers.

use std::ops::Range;
use std::sync::Arc;

use core_contracts::OutlineNode;
use core_contracts::{PaneKind, PaneOutlineHost, PaneRenderContext, PaneView};
use core_contracts::{SearchMatch, SearchQuery};
use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window};
use theme::Theme;

use crate::node::PreviewBlock;
use crate::selection::{PreviewEndpoint, PreviewSelectionRange};

/// Read-only block tree shown in the preview panel.
#[derive(Default)]
pub struct PreviewState {
    pub blocks: Vec<PreviewBlock>,
    pub selection: Option<PreviewSelectionRange>,
    pub drag_anchor: Option<PreviewEndpoint>,
    pub search_matches: Vec<(Range<usize>, bool)>,
    pub source_hash: u64,
    /// Document revision the preview tree was last synced at; `None` until
    /// the first build.
    pub synced_revision: Option<u64>,
}

impl PaneView for PreviewState {
    fn kind(&self) -> PaneKind {
        PaneKind::from_static(crate::builder::PANE_KIND)
    }

    fn capabilities(&self) -> core_contracts::PaneCapabilities {
        core_contracts::PaneCapabilities {
            editable: false,
            searchable: true,
            replaceable: false,
            outline: true,
            navigable: true,
        }
    }

    fn sync_document(&mut self, document: &core_contracts::DocumentSnapshot, _cx: &mut App) {
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
            &Default::default(),
            &footnote_registry,
        );
        self.blocks = roots;
        self.source_hash = hash;
        self.synced_revision = Some(revision);
    }

    fn serialize_text(&self, _cx: &App) -> Option<String> {
        let text = self
            .blocks
            .iter()
            .map(|b| b.display_text().to_string())
            .collect::<Vec<_>>()
            .join("\n\n");
        Some(text)
    }

    fn outline_headings(&self, _cx: &App) -> Vec<OutlineNode> {
        let text = self
            .blocks
            .iter()
            .map(|b| b.display_text().to_string())
            .collect::<Vec<_>>()
            .join("\n\n");
        crate::outline::extract_outline_headings(&text)
    }

    fn navigate_to_outline(&mut self, index: usize, theme: &Theme, _cx: &mut App) {
        let headings = self.outline_headings(_cx);
        if let Some(node) = headings.get(index) {
            let font_size = theme.typography.text_size.max(14.0);
            let line_height = (font_size * theme.typography.text_line_height)
                .round()
                .max(22.0);
            let _target_y =
                crate::outline::calculate_scroll_offset_for_node(self, node, line_height);
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

    fn render(&mut self, ctx: &PaneRenderContext, window: &mut Window, cx: &mut App) -> AnyElement {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let strings = config::language::I18nStrings::en_us();
        let preview_body =
            crate::render::render_preview_pane(self, ctx, &theme, &strings, window, cx);
        let outline_host: Arc<dyn core_contracts::OutlineHost> = Arc::new(PaneOutlineHost {
            pane_id: ctx.pane_id,
            host: ctx.host.clone(),
        });
        let outline_hud = ui::render_floating_outline_hud(
            ctx.pane_id.0,
            &self.outline_headings(cx),
            None,
            false,
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

    fn handle_navigation(
        &mut self,
        target: &core_contracts::NavigationTarget,
        _modifiers: gpui::Modifiers,
        _cx: &mut gpui::App,
    ) -> Option<core_contracts::NavigationExecutionPlan> {
        match target {
            core_contracts::NavigationTarget::External { resolved, .. } => Some(
                core_contracts::NavigationExecutionPlan::OpenExternalUrl(resolved.clone()),
            ),
            core_contracts::NavigationTarget::FootnoteDefinition { id } => Some(
                core_contracts::NavigationExecutionPlan::ScrollToFootnote(id.clone()),
            ),
            core_contracts::NavigationTarget::FootnoteReference { id } => {
                Some(core_contracts::NavigationExecutionPlan::ScrollToFootnoteRef(id.clone()))
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
