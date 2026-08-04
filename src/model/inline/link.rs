//! Link metadata attached to inline text fragments.
//!
//! Links are stored as enum variants so the serializer can reconstruct
//! the correct Markdown syntax (inline, reference, or autolink).

/// Link metadata attached to a formatted inline text fragment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineLink {
    /// Inline destination and optional title: `[label](url "title")`.
    Inline {
        destination: String,
        title: Option<String>,
    },
    /// Reference-style link resolved from `[label][ref]` syntax.
    Reference {
        label: String,
        destination: String,
    },
    /// Autolink from `<scheme:target>` or email-like syntax.
    Autolink {
        target: String,
    },
}

/// Link target pair used by hit-testing and open-link prompts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineLinkHit {
    /// Raw source displayed to the user.
    pub prompt_target: String,
    /// Resolved target to open.
    pub open_target: String,
}

impl InlineLink {
    /// The resolved destination URL for this link.
    pub fn open_target(&self) -> &str {
        match self {
            Self::Inline { destination, .. } | Self::Reference { destination, .. } => destination,
            Self::Autolink { target } => target,
        }
    }

    /// The raw target as written in source (label for reference links).
    pub fn raw_target(&self) -> &str {
        match self {
            Self::Inline { destination, .. } => destination,
            Self::Reference { label, .. } => label,
            Self::Autolink { target } => target,
        }
    }

    /// Build a hit-test payload from this link.
    pub(crate) fn hit(&self) -> InlineLinkHit {
        InlineLinkHit {
            prompt_target: self.raw_target().to_string(),
            open_target: self.open_target().to_string(),
        }
    }

    /// Whether the link syntax is source-preserving (cannot be losslessly
    /// reconstructed from plain text alone).
    pub(crate) fn is_source_preserving(&self) -> bool {
        matches!(self, Self::Reference { .. } | Self::Autolink { .. })
    }

    /// Opening marker for Markdown serialization.
    pub(crate) fn open_marker(&self) -> &'static str {
        match self {
            Self::Autolink { .. } => "<",
            Self::Inline { .. } | Self::Reference { .. } => "[",
        }
    }

    /// Middle marker between label and target (None for autolinks).
    pub(crate) fn middle_marker(&self) -> Option<&'static str> {
        match self {
            Self::Inline { .. } => Some("]("),
            Self::Reference { .. } => Some("]["),
            Self::Autolink { .. } => None,
        }
    }

    /// The editable portion of the link syntax (URL + optional title, or reference label).
    pub(crate) fn editable_text(&self) -> Option<String> {
        match self {
            Self::Inline { destination, title } => {
                Some(format_inline_link_target(destination, title.as_deref()))
            }
            Self::Reference { label, .. } => Some(label.clone()),
            Self::Autolink { .. } => None,
        }
    }

    /// Closing marker for Markdown serialization.
    pub(crate) fn close_marker(&self) -> &'static str {
        match self {
            Self::Inline { .. } => ")",
            Self::Reference { .. } => "]",
            Self::Autolink { .. } => ">",
        }
    }
}

fn format_inline_link_target(destination: &str, title: Option<&str>) -> String {
    match title {
        Some(title) => format!("{destination} \"{}\"", escape_link_title(title)),
        None => destination.to_string(),
    }
}

fn escape_link_title(title: &str) -> String {
    let mut escaped = String::with_capacity(title.len());
    for ch in title.chars() {
        if matches!(ch, '\\' | '"') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}
