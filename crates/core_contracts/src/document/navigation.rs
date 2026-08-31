//! Document navigation targets and execution plans.

/// Target of an in-document or external navigation action.
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

/// Resolved plan describing what action the editor container should execute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationExecutionPlan {
    OpenExternalUrl(String),
    PromptAndOpenExternalUrl {
        prompt_target: String,
        open_target: String,
    },
    ScrollToFootnote(String),
    ScrollToFootnoteRef(String),
    JumpToFootnoteDef(String),
    JumpToFootnoteRef(String),
}
