use gpui::EntityId;

/// Category of an outline item (Markdown heading or code symbol).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OutlineNodeKind {
    #[default]
    Heading,
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Module,
    Constant,
    Section,
}

impl OutlineNodeKind {
    /// Short tag display for the symbol (e.g. "fn", "struct", "class").
    pub fn badge_label(&self) -> Option<&'static str> {
        match self {
            Self::Heading => None,
            Self::Function => Some("fn"),
            Self::Method => Some("m"),
            Self::Class => Some("class"),
            Self::Struct => Some("struct"),
            Self::Enum => Some("enum"),
            Self::Trait => Some("trait"),
            Self::Module => Some("mod"),
            Self::Constant => Some("const"),
            Self::Section => Some("sec"),
        }
    }
}

/// A single heading or code symbol item in the outline TOC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineNode {
    /// Unique identifier for the element in the outline.
    pub id: String,
    /// Display label of the heading or symbol.
    pub label: String,
    /// Heading level (1..=6) or nesting level for code symbols.
    pub level: u8,
    /// 0-indexed line index (in SourceCode/Preview) or block index (in WYSIWYG).
    pub block_index: usize,
    /// Logical EntityId if in active WYSIWYG document.
    pub block_id: Option<EntityId>,
    /// The specific symbol kind of this outline node.
    pub kind: OutlineNodeKind,
}

impl OutlineNode {
    pub fn heading(id: String, label: String, level: u8, block_index: usize) -> Self {
        Self {
            id,
            label,
            level,
            block_index,
            block_id: None,
            kind: OutlineNodeKind::Heading,
        }
    }

    pub fn symbol(
        id: String,
        label: String,
        level: u8,
        line: usize,
        kind: OutlineNodeKind,
    ) -> Self {
        Self {
            id,
            label,
            level,
            block_index: line,
            block_id: None,
            kind,
        }
    }
}

/// Outline HUD state attached to an Editor.
#[derive(Clone, Debug, Default)]
pub struct OutlineHudState {
    /// Whether the hover TOC popover card is currently visible.
    pub is_hovered: bool,
    /// Whether the docked right-side outline bar is open.
    pub is_docked_open: bool,
    /// Index of the heading currently in the active viewport.
    pub active_index: Option<usize>,
    /// Debounce generation token for mouse leave closure.
    pub close_token: usize,
}
