//! Code-block syntax highlighting support.

use std::ops::Range;
#[cfg(feature = "code-highlight-core")]
use std::sync::{Arc, LazyLock};

use gpui::{Font, FontStyle, FontWeight, Hsla, TextRun, UnderlineStyle, px};

use crate::engine::{HighlightMap, LanguageConfig};

use theme::ThemeColors;

/// Canonical language key used by the syntax-highlighting registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeLanguageKey {
    /// Rust source code.
    Rust,
    /// JavaScript without JSX.
    JavaScript,
    /// JavaScript with JSX syntax.
    JavaScriptJsx,
    /// TypeScript without TSX.
    TypeScript,
    /// TypeScript with TSX syntax.
    TypeScriptTsx,
    /// JSON data.
    Json,
    /// Markdown source.
    Markdown,
    /// Markdown inline content (injection-only language).
    MarkdownInline,
    /// POSIX-like shell scripts.
    Bash,
    /// C source code.
    C,
    /// C++ source code.
    Cpp,
    /// C# source code.
    CSharp,
    /// CSS stylesheets.
    Css,
    /// Go source code.
    Go,
    /// HTML markup.
    Html,
    /// Java source code.
    Java,
    /// PHP source code.
    Php,
    /// Python source code.
    Python,
    /// Ruby source code.
    Ruby,
    /// YAML structured data.
    Yaml,
    /// TOML configuration data.
    Toml,
    /// Plain text without syntax highlighting.
    PlainText,
    /// Mermaid diagram source.
    Mermaid,
    /// LaTeX math expression source.
    Latex,
}

/// Token class and span vocabulary, owned by the document contracts and
/// re-exported here for consumers of the highlighter.
pub use editor_contracts::{CodeHighlightClass, CodeHighlightSpan};

/// Highlight result cached on a code block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeHighlightResult {
    pub language: CodeLanguageKey,
    pub spans: Vec<CodeHighlightSpan>,
}

/// Language aliases accepted from fenced-code info strings.
#[derive(Clone, Copy)]
struct LanguageDescriptor {
    key: CodeLanguageKey,
    aliases: &'static [&'static str],
}

const LANGUAGE_DESCRIPTORS: &[LanguageDescriptor] = &[
    LanguageDescriptor {
        key: CodeLanguageKey::Rust,
        aliases: &["rust", "rs"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::JavaScript,
        aliases: &["javascript", "js"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::JavaScriptJsx,
        aliases: &["jsx"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::TypeScript,
        aliases: &["typescript", "ts"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::TypeScriptTsx,
        aliases: &["tsx"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Json,
        aliases: &["json"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Markdown,
        aliases: &["markdown", "md"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::MarkdownInline,
        aliases: &["markdown_inline", "markdown-inline"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Bash,
        aliases: &["bash", "sh", "shell", "zsh"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::C,
        aliases: &["c", "h"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Cpp,
        aliases: &["cpp", "cxx", "cc", "hpp", "hxx"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::CSharp,
        aliases: &["csharp", "cs", "c#"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Css,
        aliases: &["css"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Go,
        aliases: &["go", "golang"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Html,
        aliases: &["html"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Java,
        aliases: &["java"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Php,
        aliases: &["php"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Python,
        aliases: &["python", "py"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Ruby,
        aliases: &["ruby", "rb"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Yaml,
        aliases: &["yaml", "yml"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Toml,
        aliases: &["toml"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::PlainText,
        aliases: &["text", "txt", "plain"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Mermaid,
        aliases: &["mermaid"],
    },
    LanguageDescriptor {
        key: CodeLanguageKey::Latex,
        aliases: &["latex", "math", "tex"],
    },
];

/// Builds a language configuration from its grammar and queries.
#[cfg(feature = "code-highlight-core")]
fn language_config_of(
    name: &'static str,
    grammar: fn() -> tree_sitter::Language,
    highlights_query: &'static str,
    injections_query: &'static str,
) -> LanguageConfig {
    LanguageConfig {
        name,
        grammar,
        highlights_query,
        injections_query,
    }
}

#[cfg(feature = "code-highlight-core")]
fn build_language_config(key: CodeLanguageKey) -> Option<LanguageConfig> {
    Some(match key {
        CodeLanguageKey::Rust => language_config_of(
            "rust",
            || tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            tree_sitter_rust::INJECTIONS_QUERY,
        ),
        CodeLanguageKey::JavaScript => language_config_of(
            "javascript",
            || tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::INJECTIONS_QUERY,
        ),
        CodeLanguageKey::JavaScriptJsx => {
            static JSX_QUERY: LazyLock<String> = LazyLock::new(|| {
                format!(
                    "{}\n{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
                )
            });
            language_config_of(
                "javascript",
                || tree_sitter_javascript::LANGUAGE.into(),
                JSX_QUERY.as_str(),
                tree_sitter_javascript::INJECTIONS_QUERY,
            )
        }
        CodeLanguageKey::TypeScript => language_config_of(
            "typescript",
            || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            "",
        ),
        CodeLanguageKey::TypeScriptTsx => language_config_of(
            "tsx",
            || tree_sitter_typescript::LANGUAGE_TSX.into(),
            tree_sitter_typescript::HIGHLIGHTS_QUERY,
            "",
        ),
        CodeLanguageKey::Json => language_config_of(
            "json",
            || tree_sitter_json::LANGUAGE.into(),
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Markdown => language_config_of(
            "markdown",
            || tree_sitter_md::LANGUAGE.into(),
            MARKDOWN_HIGHLIGHT_QUERY,
            MARKDOWN_INJECTION_QUERY,
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::MarkdownInline => language_config_of(
            "markdown_inline",
            || tree_sitter_md::INLINE_LANGUAGE.into(),
            tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
            tree_sitter_md::INJECTION_QUERY_INLINE,
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Bash => language_config_of(
            "bash",
            || tree_sitter_bash::LANGUAGE.into(),
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::C => language_config_of(
            "c",
            || tree_sitter_c::LANGUAGE.into(),
            tree_sitter_c::HIGHLIGHT_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Cpp => language_config_of(
            "cpp",
            || tree_sitter_cpp::LANGUAGE.into(),
            tree_sitter_cpp::HIGHLIGHT_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::CSharp => language_config_of(
            "c_sharp",
            || tree_sitter_c_sharp::LANGUAGE.into(),
            tree_sitter_c_sharp::HIGHLIGHTS_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Css => language_config_of(
            "css",
            || tree_sitter_css::LANGUAGE.into(),
            tree_sitter_css::HIGHLIGHTS_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Go => language_config_of(
            "go",
            || tree_sitter_go::LANGUAGE.into(),
            tree_sitter_go::HIGHLIGHTS_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Html => language_config_of(
            "html",
            || tree_sitter_html::LANGUAGE.into(),
            tree_sitter_html::HIGHLIGHTS_QUERY,
            tree_sitter_html::INJECTIONS_QUERY,
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Java => language_config_of(
            "java",
            || tree_sitter_java::LANGUAGE.into(),
            tree_sitter_java::HIGHLIGHTS_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Php => language_config_of(
            "php",
            || tree_sitter_php::LANGUAGE_PHP.into(),
            tree_sitter_php::HIGHLIGHTS_QUERY,
            tree_sitter_php::INJECTIONS_QUERY,
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Python => language_config_of(
            "python",
            || tree_sitter_python::LANGUAGE.into(),
            tree_sitter_python::HIGHLIGHTS_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Ruby => language_config_of(
            "ruby",
            || tree_sitter_ruby::LANGUAGE.into(),
            tree_sitter_ruby::HIGHLIGHTS_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-config")]
        CodeLanguageKey::Yaml => language_config_of(
            "yaml",
            || tree_sitter_yaml::LANGUAGE.into(),
            tree_sitter_yaml::HIGHLIGHTS_QUERY,
            "",
        ),
        #[cfg(feature = "code-highlight-config")]
        CodeLanguageKey::Toml => language_config_of(
            "toml",
            || tree_sitter_toml::LANGUAGE.into(),
            tree_sitter_toml::HIGHLIGHTS_QUERY,
            "",
        ),
        _ => return None,
    })
}

/// Lazily-built, per-language cached configuration.
#[cfg(feature = "code-highlight-core")]
pub fn language_config(key: CodeLanguageKey) -> Option<Arc<LanguageConfig>> {
    macro_rules! cached {
        ($key:expr) => {{
            static CONFIG: LazyLock<Option<Arc<LanguageConfig>>> =
                LazyLock::new(|| build_language_config($key).map(Arc::new));
            CONFIG.clone()
        }};
    }
    match key {
        CodeLanguageKey::Rust => cached!(CodeLanguageKey::Rust),
        CodeLanguageKey::JavaScript => cached!(CodeLanguageKey::JavaScript),
        CodeLanguageKey::JavaScriptJsx => cached!(CodeLanguageKey::JavaScriptJsx),
        CodeLanguageKey::TypeScript => cached!(CodeLanguageKey::TypeScript),
        CodeLanguageKey::TypeScriptTsx => cached!(CodeLanguageKey::TypeScriptTsx),
        CodeLanguageKey::Json => cached!(CodeLanguageKey::Json),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Markdown => cached!(CodeLanguageKey::Markdown),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::MarkdownInline => cached!(CodeLanguageKey::MarkdownInline),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Bash => cached!(CodeLanguageKey::Bash),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::C => cached!(CodeLanguageKey::C),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Cpp => cached!(CodeLanguageKey::Cpp),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::CSharp => cached!(CodeLanguageKey::CSharp),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Css => cached!(CodeLanguageKey::Css),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Go => cached!(CodeLanguageKey::Go),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Html => cached!(CodeLanguageKey::Html),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Java => cached!(CodeLanguageKey::Java),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Php => cached!(CodeLanguageKey::Php),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Python => cached!(CodeLanguageKey::Python),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Ruby => cached!(CodeLanguageKey::Ruby),
        #[cfg(feature = "code-highlight-config")]
        CodeLanguageKey::Yaml => cached!(CodeLanguageKey::Yaml),
        #[cfg(feature = "code-highlight-config")]
        CodeLanguageKey::Toml => cached!(CodeLanguageKey::Toml),
        _ => None,
    }
}

/// Eagerly compiles every language's grammar queries, so the first edit
/// does not pay their lazy-initialization cost. Called once from a
/// background thread at startup.
#[cfg(feature = "code-highlight-core")]
pub fn prewarm_code_highlight_registry() {
    let keys = [
        CodeLanguageKey::Rust,
        CodeLanguageKey::JavaScript,
        CodeLanguageKey::JavaScriptJsx,
        CodeLanguageKey::TypeScript,
        CodeLanguageKey::TypeScriptTsx,
        CodeLanguageKey::Json,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Markdown,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::MarkdownInline,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Bash,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::C,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Cpp,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::CSharp,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Css,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Go,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Html,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Java,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Php,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Python,
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Ruby,
        #[cfg(feature = "code-highlight-config")]
        CodeLanguageKey::Yaml,
        #[cfg(feature = "code-highlight-config")]
        CodeLanguageKey::Toml,
    ];
    for key in keys {
        let _ = language_config(key);
    }
}

#[cfg(feature = "code-highlight-official")]
/// Markdown block-grammar highlight query with per-level heading captures
/// and block markup classes. Inline markup (emphasis, links, code spans)
/// arrives through the `markdown_inline` injection, whose spans overlay
/// these block spans.
const MARKDOWN_HIGHLIGHT_QUERY: &str = r#"
; Heading text and markers, per level.
(atx_heading (atx_h1_marker) (inline) @markup.heading.1)
(atx_heading (atx_h2_marker) (inline) @markup.heading.2)
(atx_heading (atx_h3_marker) (inline) @markup.heading.3)
(atx_heading (atx_h4_marker) (inline) @markup.heading.4)
(atx_heading (atx_h5_marker) (inline) @markup.heading.5)
(atx_heading (atx_h6_marker) (inline) @markup.heading.6)
(atx_heading (atx_h1_marker) @markup.heading.1)
(atx_heading (atx_h2_marker) @markup.heading.2)
(atx_heading (atx_h3_marker) @markup.heading.3)
(atx_heading (atx_h4_marker) @markup.heading.4)
(atx_heading (atx_h5_marker) @markup.heading.5)
(atx_heading (atx_h6_marker) @markup.heading.6)
(setext_heading (paragraph) @markup.heading.1 (setext_h1_underline))
(setext_heading (paragraph) @markup.heading.2 (setext_h2_underline))
(setext_heading (setext_h1_underline) @markup.heading.1)
(setext_heading (setext_h2_underline) @markup.heading.2)

; List markers, thematic breaks, and quote markers.
[
  (list_marker_plus)
  (list_marker_minus)
  (list_marker_star)
  (list_marker_dot)
  (list_marker_parenthesis)
] @markup.list
(thematic_break) @markup.list
(block_quote_marker) @markup.quote

; Fences and escapes.
(fenced_code_block_delimiter) @punctuation.delimiter
(backslash_escape) @markup.escape
"#;

/// Markdown injection query: fenced code blocks inject their info-string
/// language, HTML blocks inject `html`, and inline content injects
/// `markdown_inline`. The inline injection sets
/// `injection.include-children` because the `inline` node's text lives in
/// its children.
const MARKDOWN_INJECTION_QUERY: &str = r#"
(fenced_code_block
  (info_string
    (language) @injection.language)
  (code_fence_content) @injection.content)

((html_block) @injection.content
  (#set! injection.language "html"))

((inline) @injection.content
  (#set! injection.language "markdown_inline")
  (#set! injection.include-children))
"#;

fn descriptor_for_language(language: &str) -> Option<&'static LanguageDescriptor> {
    LANGUAGE_DESCRIPTORS.iter().find(|descriptor| {
        descriptor
            .aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(language))
    })
}

pub fn resolve_code_language_key(language: Option<&str>) -> Option<CodeLanguageKey> {
    let normalized = language?
        .split_whitespace()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    descriptor_for_language(normalized).map(|descriptor| descriptor.key)
}

pub fn highlight_code_block(language: Option<&str>, source: &str) -> Option<CodeHighlightResult> {
    let key = resolve_code_language_key(language)?;

    // One-shot full parse through the incremental engine: the same code
    // path the live editors use, just without edit reuse.
    #[cfg(feature = "code-highlight-core")]
    if let Some(map) = HighlightMap::new(key, source) {
        return Some(CodeHighlightResult {
            language: key,
            spans: map.into_flat_spans(),
        });
    }

    // Languages without a tree-sitter grammar (LaTeX, Mermaid) fall back to
    // lightweight rule-based spans so math and diagram blocks colorize while
    // editing, mirroring the code-block experience.
    if let Some(spans) = highlight_light_rules(key, source) {
        return Some(CodeHighlightResult {
            language: key,
            spans,
        });
    }

    Some(CodeHighlightResult {
        language: key,
        spans: Vec::new(),
    })
}

/// Lightweight rule-based highlighting for languages without a tree-sitter
/// grammar. Returns `None` for languages that have no light rules either.
fn highlight_light_rules(key: CodeLanguageKey, source: &str) -> Option<Vec<CodeHighlightSpan>> {
    match key {
        CodeLanguageKey::Latex => Some(highlight_latex_text(source)),
        CodeLanguageKey::Mermaid => Some(highlight_mermaid_text(source)),
        _ => None,
    }
}

/// LaTeX light rules: `%` line comments, `\command` names, numeric literals.
fn highlight_latex_text(source: &str) -> Vec<CodeHighlightSpan> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'%' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            push_highlight_span(&mut spans, start..i, CodeHighlightClass::Comment);
            continue;
        }
        if byte == b'\\'
            && bytes
                .get(i + 1)
                .is_some_and(|next| next.is_ascii_alphabetic())
        {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            push_highlight_span(&mut spans, start..i, CodeHighlightClass::Function);
            continue;
        }
        if byte.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            push_highlight_span(&mut spans, start..i, CodeHighlightClass::Number);
            continue;
        }
        i += 1;
    }
    spans
}

/// Keywords recognized by the light Mermaid highlighter.
const MERMAID_KEYWORDS: &[&str] = &[
    "graph",
    "flowchart",
    "sequenceDiagram",
    "classDiagram",
    "stateDiagram",
    "erDiagram",
    "journey",
    "pie",
    "gantt",
    "mindmap",
    "timeline",
    "gitGraph",
    "subgraph",
    "end",
    "direction",
    "TB",
    "TD",
    "LR",
    "RL",
    "BT",
    "accTitle",
    "accDescr",
];

/// Mermaid light rules: `%%` line comments, diagram keywords, edge arrows,
/// and node bracket punctuation.
fn highlight_mermaid_text(source: &str) -> Vec<CodeHighlightSpan> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'%' && bytes.get(i + 1) == Some(&b'%') {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            push_highlight_span(&mut spans, start..i, CodeHighlightClass::Comment);
            continue;
        }
        if byte.is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if MERMAID_KEYWORDS
                .iter()
                .any(|keyword| *keyword == &source[start..i])
            {
                push_highlight_span(&mut spans, start..i, CodeHighlightClass::Keyword);
            }
            continue;
        }
        if matches!(byte, b'-' | b'=' | b'~' | b'.') {
            let start = i;
            while i < bytes.len()
                && matches!(bytes[i], b'-' | b'=' | b'~' | b'.' | b'>' | b'x' | b'o')
            {
                i += 1;
            }
            if i > start + 1 {
                push_highlight_span(&mut spans, start..i, CodeHighlightClass::Operator);
            }
            continue;
        }
        if matches!(byte, b'[' | b']' | b'(' | b')' | b'{' | b'}') {
            push_highlight_span(&mut spans, i..i + 1, CodeHighlightClass::Punctuation);
            i += 1;
            continue;
        }
        i += 1;
    }
    spans
}

fn push_highlight_span(
    spans: &mut Vec<CodeHighlightSpan>,
    range: Range<usize>,
    class: CodeHighlightClass,
) {
    if range.start >= range.end {
        return;
    }

    if let Some(last) = spans.last_mut()
        && last.class == class
        && last.range.end == range.start
    {
        last.range.end = range.end;
        return;
    }

    spans.push(CodeHighlightSpan { range, class });
}

/// Maps a tree-sitter capture name to a highlight class. Unknown captures
/// are ignored, so grammars' extra captures stay uncolored.
#[cfg(feature = "code-highlight-core")]
pub fn class_for_highlight(name: &str) -> Option<CodeHighlightClass> {
    Some(match name {
        "comment" => CodeHighlightClass::Comment,
        "keyword" | "tag" => CodeHighlightClass::Keyword,
        "string" | "string.special" | "embedded" => CodeHighlightClass::String,
        "number" => CodeHighlightClass::Number,
        "type" | "type.builtin" | "module" => CodeHighlightClass::Type,
        "function" | "function.builtin" | "constructor" => CodeHighlightClass::Function,
        "constant" | "constant.builtin" => CodeHighlightClass::Constant,
        "variable" | "variable.builtin" | "variable.parameter" => CodeHighlightClass::Variable,
        "property" | "property.builtin" | "attribute" => CodeHighlightClass::Property,
        "operator" => CodeHighlightClass::Operator,
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" | "punctuation.special" => {
            CodeHighlightClass::Punctuation
        }
        "markup.heading.1" => CodeHighlightClass::MarkupHeading(1),
        "markup.heading.2" => CodeHighlightClass::MarkupHeading(2),
        "markup.heading.3" => CodeHighlightClass::MarkupHeading(3),
        "markup.heading.4" => CodeHighlightClass::MarkupHeading(4),
        "markup.heading.5" => CodeHighlightClass::MarkupHeading(5),
        "markup.heading.6" => CodeHighlightClass::MarkupHeading(6),
        "markup.bold" => CodeHighlightClass::MarkupBold,
        "markup.italic" => CodeHighlightClass::MarkupItalic,
        "markup.code" => CodeHighlightClass::MarkupCode,
        "markup.link" => CodeHighlightClass::MarkupLink,
        "markup.uri" => CodeHighlightClass::MarkupUri,
        "markup.list" => CodeHighlightClass::MarkupList,
        "markup.quote" => CodeHighlightClass::MarkupQuote,
        "markup.escape" | "string.escape" => CodeHighlightClass::MarkupEscape,
        "text.literal" => CodeHighlightClass::MarkupCode,
        "text.emphasis" => CodeHighlightClass::MarkupItalic,
        "text.strong" => CodeHighlightClass::MarkupBold,
        "text.uri" => CodeHighlightClass::MarkupUri,
        "text.reference" => CodeHighlightClass::MarkupLink,
        _ => return None,
    })
}

pub fn code_highlight_color(colors: &ThemeColors, class: CodeHighlightClass) -> Hsla {
    match class {
        CodeHighlightClass::Comment => colors.code_syntax_comment,
        CodeHighlightClass::Keyword => colors.code_syntax_keyword,
        CodeHighlightClass::String => colors.code_syntax_string,
        CodeHighlightClass::Number => colors.code_syntax_number,
        CodeHighlightClass::Type => colors.code_syntax_type,
        CodeHighlightClass::Function => colors.code_syntax_function,
        CodeHighlightClass::Constant => colors.code_syntax_constant,
        CodeHighlightClass::Variable => colors.code_syntax_variable,
        CodeHighlightClass::Property => colors.code_syntax_property,
        CodeHighlightClass::Operator => colors.code_syntax_operator,
        CodeHighlightClass::Punctuation => colors.code_syntax_punctuation,
        CodeHighlightClass::MarkupHeading(1) => colors.text_h1,
        CodeHighlightClass::MarkupHeading(2) => colors.text_h2,
        CodeHighlightClass::MarkupHeading(3) => colors.text_h3,
        CodeHighlightClass::MarkupHeading(4) => colors.text_h4,
        CodeHighlightClass::MarkupHeading(5) => colors.text_h5,
        CodeHighlightClass::MarkupHeading(_) => colors.text_h6,
        CodeHighlightClass::MarkupCode => colors.code_text,
        CodeHighlightClass::MarkupLink | CodeHighlightClass::MarkupUri => colors.text_link,
        CodeHighlightClass::MarkupList => colors.markdown_marker,
        CodeHighlightClass::MarkupQuote => colors.text_quote,
        CodeHighlightClass::MarkupEscape => colors.code_syntax_string,
        CodeHighlightClass::MarkupBold | CodeHighlightClass::MarkupItalic => colors.text_default,
    }
}

/// Builds a sequence of `TextRun`s for a single line using Tree-sitter
/// highlight spans. Markdown markup classes get Zed-style styling: heading
/// colors and weights, bold/italic faces, tinted inline-code backgrounds,
/// and underlined links.
pub fn build_line_text_runs(
    line_text: &str,
    line_range: Range<usize>,
    spans: &[CodeHighlightSpan],
    font: Font,
    theme_colors: &ThemeColors,
) -> Vec<TextRun> {
    if line_text.is_empty() {
        return Vec::new();
    }

    if spans.is_empty() {
        return vec![TextRun {
            len: line_text.len(),
            font,
            color: theme_colors.text_default,
            ..Default::default()
        }];
    }

    let l_start = line_range.start;
    let l_end = line_range.end;
    let mut runs = Vec::new();
    let mut current_offset = 0; // relative to line start (0..line_text.len())

    for span in spans {
        if span.range.end <= l_start || span.range.start >= l_end {
            continue;
        }

        let span_local_start = span
            .range
            .start
            .saturating_sub(l_start)
            .min(line_text.len());
        let span_local_end = span.range.end.saturating_sub(l_start).min(line_text.len());

        if span_local_start > current_offset {
            let gap_len = span_local_start - current_offset;
            runs.push(TextRun {
                len: gap_len,
                font: font.clone(),
                color: theme_colors.text_default,
                ..Default::default()
            });
            current_offset = span_local_start;
        }

        if span_local_end > current_offset {
            let seg_len = span_local_end - current_offset;
            let mut run_font = font.clone();
            let mut run = TextRun {
                len: seg_len,
                font: run_font.clone(),
                color: code_highlight_color(theme_colors, span.class),
                ..Default::default()
            };
            match span.class {
                CodeHighlightClass::MarkupHeading(_) | CodeHighlightClass::MarkupBold => {
                    run_font.weight = FontWeight::BOLD;
                }
                CodeHighlightClass::MarkupItalic => {
                    run_font.style = FontStyle::Italic;
                }
                CodeHighlightClass::MarkupCode => {
                    run.background_color = Some(theme_colors.code_bg.opacity(0.6));
                }
                CodeHighlightClass::MarkupLink => {
                    run.underline = Some(UnderlineStyle {
                        color: Some(theme_colors.text_link),
                        thickness: px(1.0),
                        wavy: false,
                    });
                }
                _ => {}
            }
            run.font = run_font;
            runs.push(run);
            current_offset = span_local_end;
        }
    }

    if current_offset < line_text.len() {
        runs.push(TextRun {
            len: line_text.len() - current_offset,
            font: font.clone(),
            color: theme_colors.text_default,
            ..Default::default()
        });
    }

    if runs.is_empty() {
        runs.push(TextRun {
            len: line_text.len(),
            font,
            color: theme_colors.text_default,
            ..Default::default()
        });
    }

    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn markdown_query_produces_markup_classes() {
        let source = "# Heading One\n\nSome **bold** and *italic* text with `code` and a [link](https://example.com).\n\n- item one\n- item two\n\n> quoted line\n\n```rust\nfn main() {}\n```\n";
        let result = highlight_code_block(Some("markdown"), source).expect("markdown highlight");
        let classes: HashSet<_> = result.spans.iter().map(|span| span.class).collect();
        assert!(
            classes.contains(&CodeHighlightClass::MarkupHeading(1)),
            "h1"
        );
        assert!(classes.contains(&CodeHighlightClass::MarkupBold), "bold");
        assert!(
            classes.contains(&CodeHighlightClass::MarkupItalic),
            "italic"
        );
        assert!(
            classes.contains(&CodeHighlightClass::MarkupCode),
            "inline code"
        );
        assert!(
            classes.contains(&CodeHighlightClass::MarkupLink),
            "link text"
        );
        assert!(classes.contains(&CodeHighlightClass::MarkupUri), "link uri");
        assert!(
            classes.contains(&CodeHighlightClass::MarkupList),
            "list markers"
        );
        assert!(
            classes.contains(&CodeHighlightClass::MarkupQuote),
            "quote marker"
        );
        // The fenced Rust block is injected and highlighted with the inner
        // language, producing code classes.
        assert!(
            classes.contains(&CodeHighlightClass::Keyword),
            "rust keyword"
        );
    }
}
