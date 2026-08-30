//! Unified navigation engine — zero compatibility, single source of truth.

use gpui::*;

/// Target of a navigation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationTarget {
    /// External web URL or local file target.
    External {
        raw: String,
        resolved: String,
    },
    /// In-document footnote definition.
    FootnoteDefinition { id: String },
    /// In-document footnote back-reference.
    FootnoteReference { id: String },
}

/// The mode the navigation was initiated from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationMode {
    Wysiwyg,
    Preview,
    SourceCode,
}

/// A request to navigate to a target from a specific view mode and keyboard modifier state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationIntent {
    pub target: NavigationTarget,
    pub mode: NavigationMode,
    pub modifiers: Modifiers,
}

impl NavigationIntent {
    #[inline]
    pub fn new(target: NavigationTarget, mode: NavigationMode, modifiers: Modifiers) -> Self {
        Self {
            target,
            mode,
            modifiers,
        }
    }

    /// Resolves the strict execution policy for this intent.
    ///
    /// Zero backwards compatibility rules:
    /// - Preview mode:
    ///   - External URL -> DirectOpen
    ///   - FootnoteDefinition -> ScrollPreviewToFootnote
    ///   - FootnoteReference -> ScrollPreviewToFootnoteRef
    /// - WYSIWYG mode:
    ///   - External URL -> Requires secondary modifier (Ctrl/Cmd); triggers PromptAndOpen
    ///   - FootnoteDefinition -> Direct Jump in document (no modifier needed)
    ///   - FootnoteReference -> Direct Jump back in document (no modifier needed)
    ///   - Note: Double-click link open is strictly removed (word selection occurs instead).
    /// - SourceCode mode:
    ///   - External URL -> Requires secondary modifier (Ctrl/Cmd); triggers DirectOpen
    pub fn resolve_policy(&self) -> Option<NavigationExecutionPlan> {
        match self.mode {
            NavigationMode::Preview => match &self.target {
                NavigationTarget::External { resolved, .. } => {
                    Some(NavigationExecutionPlan::OpenExternalUrl(resolved.clone()))
                }
                NavigationTarget::FootnoteDefinition { id } => {
                    Some(NavigationExecutionPlan::ScrollPreviewToFootnote(id.clone()))
                }
                NavigationTarget::FootnoteReference { id } => {
                    Some(NavigationExecutionPlan::ScrollPreviewToFootnoteRef(id.clone()))
                }
            },
            NavigationMode::Wysiwyg => match &self.target {
                NavigationTarget::External { raw, resolved } => {
                    if self.modifiers.secondary() {
                        Some(NavigationExecutionPlan::PromptAndOpenExternalUrl {
                            prompt_target: raw.clone(),
                            open_target: resolved.clone(),
                        })
                    } else {
                        None
                    }
                }
                NavigationTarget::FootnoteDefinition { id } => {
                    Some(NavigationExecutionPlan::JumpToFootnoteDefInEditor(id.clone()))
                }
                NavigationTarget::FootnoteReference { id } => {
                    Some(NavigationExecutionPlan::JumpToFootnoteRefInEditor(id.clone()))
                }
            },
            NavigationMode::SourceCode => {
                if self.modifiers.secondary() {
                    match &self.target {
                        NavigationTarget::External { resolved, .. } => {
                            Some(NavigationExecutionPlan::OpenExternalUrl(resolved.clone()))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
        }
    }
}

/// Resolved plan describing exactly what action the editor should execute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationExecutionPlan {
    OpenExternalUrl(String),
    PromptAndOpenExternalUrl {
        prompt_target: String,
        open_target: String,
    },
    ScrollPreviewToFootnote(String),
    ScrollPreviewToFootnoteRef(String),
    JumpToFootnoteDefInEditor(String),
    JumpToFootnoteRefInEditor(String),
}

impl crate::editor::Editor {
    pub fn execute_navigation(
        &mut self,
        intent: NavigationIntent,
        cx: &mut Context<Self>,
    ) {
        let Some(plan) = intent.resolve_policy() else {
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
            NavigationExecutionPlan::ScrollPreviewToFootnote(_id) => {
                cx.notify();
            }
            NavigationExecutionPlan::ScrollPreviewToFootnoteRef(_id) => {
                cx.notify();
            }
            NavigationExecutionPlan::JumpToFootnoteDefInEditor(_id) => {
                cx.notify();
            }
            NavigationExecutionPlan::JumpToFootnoteRefInEditor(_id) => {
                cx.notify();
            }
        }
    }
}


