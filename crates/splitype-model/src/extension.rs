//! Pluggable syntax extension specifications for custom blocks and inlines.
//!
//! Defines [`SyntaxExtension`] and [`BlockExtension`] contracts to allow
//! custom Markdown features (such as Mermaid diagrams, LaTeX math fences,
//! Callout alerts, Admonitions, and custom containers) to be registered and
//! evaluated uniformly by the parser and serializer pipelines.

use crate::parse::BlockData;

/// Trait implemented by block-level syntax extensions.
pub trait BlockExtension: Send + Sync {
    /// Human-readable unique identifier for this extension (e.g. "mermaid", "latex_display").
    fn name(&self) -> &'static str;

    /// Quick heuristic to test whether this line could begin the extension block.
    fn can_parse_line(&self, line: &str) -> bool;

    /// Parse a region of lines starting at `start` index into a [`BlockData`] record.
    /// Returns the parsed block and the number of consumed lines, or `None` if
    /// the syntax was invalid.
    fn parse_region(&self, lines: &[String], start: usize) -> Option<(BlockData, usize)>;

    /// Serialize a [`BlockData`] instance back to canonical Markdown lines.
    fn serialize_block(&self, block: &BlockData, depth: usize) -> Option<String>;
}

/// Registry of pluggable syntax extensions.
#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: Vec<Box<dyn BlockExtension>>,
}

impl ExtensionRegistry {
    /// Creates an empty extension registry.
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    /// Registers a new block syntax extension.
    pub fn register(&mut self, extension: Box<dyn BlockExtension>) {
        self.extensions.push(extension);
    }

    /// Finds an extension capable of parsing the line at `start` index.
    pub fn find_parser<'a>(&'a self, line: &str) -> Option<&'a dyn BlockExtension> {
        self.extensions
            .iter()
            .find(|ext| ext.can_parse_line(line))
            .map(|ext| &**ext)
    }

    /// Returns the total number of registered extensions.
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Whether the registry contains any extensions.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyExtension;

    impl BlockExtension for DummyExtension {
        fn name(&self) -> &'static str {
            "dummy"
        }

        fn can_parse_line(&self, line: &str) -> bool {
            line.starts_with(":::dummy")
        }

        fn parse_region(&self, _lines: &[String], _start: usize) -> Option<(BlockData, usize)> {
            Some((BlockData::paragraph("dummy content"), 1))
        }

        fn serialize_block(&self, _block: &BlockData, _depth: usize) -> Option<String> {
            Some(":::dummy\n:::".into())
        }
    }

    #[test]
    fn test_extension_registry() {
        let mut registry = ExtensionRegistry::new();
        assert!(registry.is_empty());
        registry.register(Box::new(DummyExtension));
        assert_eq!(registry.len(), 1);

        assert!(registry.find_parser(":::dummy").is_some());
        assert!(registry.find_parser("# Header").is_none());
    }
}
