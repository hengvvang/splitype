//! Unified navigation engine and viewport scrolling — zero compatibility,
//! single source of truth.

use core_contracts::{NavigationExecutionPlan, NavigationTarget, PaneId};
use gpui::*;

/// A request to navigate to a target from a keyboard modifier state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationIntent {
    pub target: NavigationTarget,
    pub modifiers: Modifiers,
}

impl NavigationIntent {
    #[inline]
    pub fn new(target: NavigationTarget, modifiers: Modifiers) -> Self {
        Self { target, modifiers }
    }
}

impl crate::editor::Editor {
    pub fn execute_navigation(&mut self, intent: NavigationIntent, cx: &mut Context<Self>) {
        let pane = self.active_pane_state();
        let plan = if pane.pane.capabilities().navigable {
            pane.pane
                .handle_navigation(&intent.target, intent.modifiers, cx)
        } else {
            None
        }
        .or_else(|| match &intent.target {
            NavigationTarget::External { raw, resolved } => {
                if intent.modifiers.secondary() {
                    Some(NavigationExecutionPlan::PromptAndOpenExternalUrl {
                        prompt_target: raw.clone(),
                        open_target: resolved.clone(),
                    })
                } else {
                    None
                }
            }
            NavigationTarget::FootnoteDefinition { id } => {
                Some(NavigationExecutionPlan::JumpToFootnoteDef(id.clone()))
            }
            NavigationTarget::FootnoteReference { id } => {
                Some(NavigationExecutionPlan::JumpToFootnoteRef(id.clone()))
            }
        });

        let Some(plan) = plan else {
            return;
        };

        match plan {
            NavigationExecutionPlan::OpenExternalUrl(url) => {
                cx.open_url(&url);
            }
            NavigationExecutionPlan::PromptAndOpenExternalUrl {
                prompt_target,
                open_target,
            } => {
                self.request_open_link_prompt(prompt_target, open_target, cx);
            }
            NavigationExecutionPlan::ScrollToFootnote(_id)
            | NavigationExecutionPlan::ScrollToFootnoteRef(_id)
            | NavigationExecutionPlan::JumpToFootnoteDef(_id)
            | NavigationExecutionPlan::JumpToFootnoteRef(_id) => {
                cx.notify();
            }
        }
    }

    /// Scrolls `pane_id`'s viewport vertically by `delta` pixels.
    pub(crate) fn scroll_viewport_by(
        &mut self,
        pane_id: PaneId,
        delta: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset().y + delta)
            .unwrap_or_default();
        self.set_vertical_scroll_offset(pane_id, target, window, cx);
    }

    /// Clamps and applies a vertical scroll offset to `pane_id`'s viewport.
    pub(crate) fn set_vertical_scroll_offset(
        &mut self,
        pane_id: PaneId,
        target_y: Pixels,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let max_offset_y = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.max_offset().y.max(px(0.0)))
            .unwrap_or_default();
        let mut offset = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset())
            .unwrap_or_default();
        offset.y = target_y.min(px(0.0)).max(-max_offset_y);
        let pane = self.pane_state(pane_id);
        pane.scroll.handle.set_offset(offset);
        cx.notify();
    }
}
