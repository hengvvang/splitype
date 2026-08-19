//! Inline formatting style flags and script variants.
//!
//! `InlineStyle` is a compact bitfield of boolean formatting flags that
//! represents the active inline formatting state for a span of text.
//! It is used throughout the inline text tree and render cache.

/// Emphasis delimiter character variant (`*` vs `_`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EmphasisMarker {
    #[default]
    Asterisk,
    Underscore,
}

impl EmphasisMarker {
    pub fn char(self) -> char {
        match self {
            Self::Asterisk => '*',
            Self::Underscore => '_',
        }
    }

    pub fn from_char(ch: char) -> Self {
        match ch {
            '_' => Self::Underscore,
            _ => Self::Asterisk,
        }
    }
}

/// Bitfield of active inline formatting flags for a span of text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub script: InlineScript,
    pub bold_marker: EmphasisMarker,
    pub italic_marker: EmphasisMarker,
    pub italic_outer: bool,
}

/// Vertical script style for Markdown extension syntax (`^superscript^`, `~subscript~`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InlineScript {
    #[default]
    Normal,
    Superscript,
    Subscript,
}

impl InlineStyle {
    pub fn with_bold(self) -> Self {
        Self {
            bold: true,
            bold_marker: EmphasisMarker::Asterisk,
            ..self
        }
    }

    pub fn with_bold_char(self, ch: char) -> Self {
        Self {
            bold: true,
            bold_marker: EmphasisMarker::from_char(ch),
            ..self
        }
    }

    pub fn with_italic(self) -> Self {
        Self {
            italic: true,
            italic_marker: EmphasisMarker::Asterisk,
            ..self
        }
    }

    pub fn with_italic_char(self, ch: char) -> Self {
        Self {
            italic: true,
            italic_marker: EmphasisMarker::from_char(ch),
            ..self
        }
    }

    pub fn with_underline(self) -> Self {
        Self {
            underline: true,
            ..self
        }
    }

    pub fn with_strikethrough(self) -> Self {
        Self {
            strikethrough: true,
            ..self
        }
    }

    pub fn with_code(self) -> Self {
        Self { code: true, ..self }
    }

    pub fn with_superscript(self) -> Self {
        Self {
            script: InlineScript::Superscript,
            ..self
        }
    }

    pub fn with_subscript(self) -> Self {
        Self {
            script: InlineScript::Subscript,
            ..self
        }
    }

    pub fn has_script(self) -> bool {
        self.script != InlineScript::Normal
    }
}

/// Inline style flag addressable by editing commands (toggle bold, italic, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleFlag {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
    Superscript,
    Subscript,
}

/// Applies or removes a [`StyleFlag`] on an [`InlineStyle`].
pub(crate) fn set_style_flag(style: InlineStyle, flag: StyleFlag, enabled: bool) -> InlineStyle {
    match flag {
        StyleFlag::Bold => InlineStyle {
            bold: enabled,
            bold_marker: if enabled {
                style.bold_marker
            } else {
                EmphasisMarker::Asterisk
            },
            italic_outer: if !enabled { false } else { style.italic_outer },
            ..style
        },
        StyleFlag::Italic => InlineStyle {
            italic: enabled,
            italic_marker: if enabled {
                style.italic_marker
            } else {
                EmphasisMarker::Asterisk
            },
            italic_outer: if !enabled { false } else { style.italic_outer },
            ..style
        },
        StyleFlag::Underline => InlineStyle {
            underline: enabled,
            ..style
        },
        StyleFlag::Strikethrough => InlineStyle {
            strikethrough: enabled,
            ..style
        },
        StyleFlag::Code => InlineStyle {
            code: enabled,
            ..style
        },
        StyleFlag::Superscript => {
            let script = if enabled {
                InlineScript::Superscript
            } else {
                InlineScript::Normal
            };
            InlineStyle { script, ..style }
        }
        StyleFlag::Subscript => {
            let script = if enabled {
                InlineScript::Subscript
            } else {
                InlineScript::Normal
            };
            InlineStyle { script, ..style }
        }
    }
}

/// Returns true when the style has the given flag enabled.
pub(crate) fn style_flag_enabled(style: InlineStyle, flag: StyleFlag) -> bool {
    match flag {
        StyleFlag::Bold => style.bold,
        StyleFlag::Italic => style.italic,
        StyleFlag::Underline => style.underline,
        StyleFlag::Strikethrough => style.strikethrough,
        StyleFlag::Code => style.code,
        StyleFlag::Superscript => style.script == InlineScript::Superscript,
        StyleFlag::Subscript => style.script == InlineScript::Subscript,
    }
}
