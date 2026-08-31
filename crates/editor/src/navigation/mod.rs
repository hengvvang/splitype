//! Unified navigation engine — zero compatibility, single source of truth.

pub use core_contracts::{NavigationExecutionPlan, NavigationTarget};
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
}
