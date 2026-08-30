//! Pane type identification — extensible identifiers for editor pane plugins.

/// The strongly-typed, extensible identifier of an editor pane kind.
///
/// Wraps a static string slice for zero-allocation copyability and comparison,
/// making it completely open for built-in and third-party plugins to define
/// new pane kinds without modifying the core data model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaneKindId(pub &'static str);

impl PaneKindId {
    /// Raw Markdown source code editor.
    pub const SOURCE_CODE: Self = Self("splitype.source_code");
    /// Visual block editor (WYSIWYG rendered view).
    pub const WYSIWYG: Self = Self("splitype.wysiwyg");
    /// Read-only rendered Markdown preview.
    pub const PREVIEW: Self = Self("splitype.preview");

    // PascalCase compatibility aliases
    #[allow(non_upper_case_globals)]
    pub const SourceCode: Self = Self::SOURCE_CODE;
    #[allow(non_upper_case_globals)]
    pub const Wysiwyg: Self = Self::WYSIWYG;
    #[allow(non_upper_case_globals)]
    pub const Preview: Self = Self::PREVIEW;

    #[inline]
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    #[inline]
    pub fn from_dynamic(id: &str) -> Self {
        Self(Box::leak(id.to_string().into_boxed_str()))
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        self.0
    }

    #[inline]
    pub fn is_wysiwyg(&self) -> bool {
        *self == Self::WYSIWYG
    }

    #[inline]
    pub fn is_source_code(&self) -> bool {
        *self == Self::SOURCE_CODE
    }

    #[inline]
    pub fn is_preview(&self) -> bool {
        *self == Self::PREVIEW
    }

    pub fn name(&self) -> &'static str {
        if *self == Self::WYSIWYG {
            "Wysiwyg"
        } else if *self == Self::SOURCE_CODE {
            "Source Code"
        } else if *self == Self::PREVIEW {
            "Preview"
        } else {
            self.0
        }
    }

    pub fn all() -> &'static [PaneKindId] {
        &[Self::WYSIWYG, Self::PREVIEW, Self::SOURCE_CODE]
    }
}

impl std::fmt::Display for PaneKindId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
