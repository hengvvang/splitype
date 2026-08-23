//! Callout kind enumeration for GitHub-flavored callout blocks.
//!
//! Callouts are parsed from `[!TYPE]` headers inside blockquote containers.

/// Supported callout variant parsed from `[!TYPE]` quote headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalloutKind {
    /// Informational note callout.
    Note,
    /// Helpful tip callout.
    Tip,
    /// High-emphasis important callout.
    Important,
    /// Warning callout for risky or surprising content.
    Warning,
    /// Caution callout for potentially harmful actions.
    Caution,
}

impl CalloutKind {
    /// The `[!TYPE]` marker text of this callout.
    pub fn marker(self) -> &'static str {
        match self {
            Self::Note => "NOTE",
            Self::Tip => "TIP",
            Self::Important => "IMPORTANT",
            Self::Warning => "WARNING",
            Self::Caution => "CAUTION",
        }
    }

    /// The lowercase `[!type]` marker text of this callout.
    pub fn marker_lower(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Tip => "tip",
            Self::Important => "important",
            Self::Warning => "warning",
            Self::Caution => "caution",
        }
    }

    /// The display label of this callout.
    pub fn label(self) -> &'static str {
        self.marker_lower()
    }

    /// Parse a `[!TYPE]` header line, returning the kind and trailing text.
    pub fn parse_header_line(line: &str) -> Option<(Self, String)> {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("[!")?;
        let marker_end = rest.find(']')?;
        let marker = &rest[..marker_end];
        let variant = match marker.to_ascii_uppercase().as_str() {
            "NOTE" => Self::Note,
            "TIP" => Self::Tip,
            "IMPORTANT" => Self::Important,
            "WARNING" => Self::Warning,
            "CAUTION" => Self::Caution,
            _ => return None,
        };
        let text = rest[marker_end + 1..].trim_start().to_string();
        Some((variant, text))
    }

    /// Build the `[!TYPE]` header line for this callout.
    pub fn header_markdown(self, text_markdown: &str) -> String {
        if text_markdown.trim().is_empty() {
            format!("[!{}]", self.marker())
        } else {
            format!("[!{}] {}", self.marker(), text_markdown)
        }
    }

    /// Escape a plain quote header so it cannot be mistaken for a callout
    /// header when serializing a plain blockquote.
    pub fn escape_plain_quote_header(text_markdown: &str) -> String {
        let mut lines = text_markdown.splitn(2, '\n');
        let first = lines.next().unwrap_or_default();
        let rest = lines.next();
        let escaped_first = if Self::parse_header_line(first).is_some() {
            format!("\\{first}")
        } else {
            first.to_string()
        };
        match rest {
            Some(rest) => format!("{escaped_first}\n{rest}"),
            None => escaped_first,
        }
    }
}
