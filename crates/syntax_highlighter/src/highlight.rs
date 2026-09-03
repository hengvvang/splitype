//! Code-block syntax highlighting support.

#[cfg(feature = "code-highlight-core")]
use std::collections::HashMap;
use std::ops::Range;
#[cfg(feature = "code-highlight-core")]
use std::sync::{Arc, LazyLock, RwLock};

use gpui::{Font, FontStyle, FontWeight, Hsla, TextRun, UnderlineStyle, px};
#[cfg(feature = "code-highlight-core")]
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

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

/// Token class category for code highlighting.
#[cfg_attr(not(feature = "code-highlight-core"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeHighlightClass {
    /// Source code comment.
    Comment,
    /// Language keyword.
    Keyword,
    /// String literal.
    String,
    /// Numeric literal.
    Number,
    /// Type identifier.
    Type,
    /// Function or callable identifier.
    Function,
    /// Constant identifier.
    Constant,
    /// Variable identifier.
    Variable,
    /// Object or record property.
    Property,
    /// Operator token.
    Operator,
    /// Punctuation token.
    Punctuation,
    /// Markdown heading text (level 1..=6).
    MarkupHeading(u8),
    /// Markdown strong emphasis: rendered bold.
    MarkupBold,
    /// Markdown emphasis: rendered italic.
    MarkupItalic,
    /// Markdown inline code span: tinted background.
    MarkupCode,
    /// Markdown link text: colored and underlined.
    MarkupLink,
    /// Markdown link destination / autolink URI.
    MarkupUri,
    /// Markdown list markers and thematic breaks.
    MarkupList,
    /// Markdown block-quote markers.
    MarkupQuote,
    /// Backslash escapes and hard line breaks.
    MarkupEscape,
}

/// Highlighted byte range inside a code block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeHighlightSpan {
    pub range: Range<usize>,
    pub class: CodeHighlightClass,
}

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

#[cfg(feature = "code-highlight-core")]
const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "comment",
    "constant",
    "constant.builtin",
    "constructor",
    "embedded",
    "function",
    "function.builtin",
    "keyword",
    "module",
    "number",
    "operator",
    "property",
    "property.builtin",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
    "markup.heading.1",
    "markup.heading.2",
    "markup.heading.3",
    "markup.heading.4",
    "markup.heading.5",
    "markup.heading.6",
    "markup.bold",
    "markup.italic",
    "markup.code",
    "markup.link",
    "markup.uri",
    "markup.list",
    "markup.quote",
    "markup.escape",
    "string.escape",
    "text.literal",
    "text.emphasis",
    "text.strong",
    "text.uri",
    "text.reference",
];

/// Lazily built tree-sitter highlighter registry.
#[cfg(feature = "code-highlight-core")]
struct CodeHighlightRegistry {
    configs: RwLock<HashMap<CodeLanguageKey, Arc<HighlightConfiguration>>>,
}

#[cfg(feature = "code-highlight-core")]
static CODE_HIGHLIGHT_REGISTRY: LazyLock<CodeHighlightRegistry> =
    LazyLock::new(CodeHighlightRegistry::new);

#[cfg(feature = "code-highlight-core")]
impl CodeHighlightRegistry {
    fn new() -> Self {
        Self {
            configs: RwLock::new(HashMap::new()),
        }
    }

    fn config_for(&self, key: CodeLanguageKey) -> Option<Arc<HighlightConfiguration>> {
        if let Ok(read_guard) = self.configs.read() {
            if let Some(config) = read_guard.get(&key) {
                return Some(Arc::clone(config));
            }
        }

        let config = build_config_for(key)?;
        let arc_config = Arc::new(config);

        if let Ok(mut write_guard) = self.configs.write() {
            write_guard.insert(key, Arc::clone(&arc_config));
        }

        Some(arc_config)
    }

    fn prewarm_all(&self) {
        const ALL_KEYS: &[CodeLanguageKey] = &[
            CodeLanguageKey::Rust,
            CodeLanguageKey::JavaScript,
            CodeLanguageKey::JavaScriptJsx,
            CodeLanguageKey::TypeScript,
            CodeLanguageKey::TypeScriptTsx,
            CodeLanguageKey::Json,
            CodeLanguageKey::Markdown,
            CodeLanguageKey::Bash,
            CodeLanguageKey::Python,
            CodeLanguageKey::C,
            CodeLanguageKey::Cpp,
            CodeLanguageKey::CSharp,
            CodeLanguageKey::Css,
            CodeLanguageKey::Go,
            CodeLanguageKey::Html,
            CodeLanguageKey::Java,
            CodeLanguageKey::Php,
            CodeLanguageKey::Ruby,
            CodeLanguageKey::Yaml,
            CodeLanguageKey::Toml,
        ];

        for &key in ALL_KEYS {
            let _ = self.config_for(key);
        }
    }
}

#[cfg(feature = "code-highlight-core")]
pub fn prewarm_code_highlight_registry() {
    CODE_HIGHLIGHT_REGISTRY.prewarm_all();
}

#[cfg(feature = "code-highlight-core")]
fn build_config_for(key: CodeLanguageKey) -> Option<HighlightConfiguration> {
    match key {
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Rust => build_rust_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::JavaScript => build_javascript_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::JavaScriptJsx => build_jsx_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::TypeScript => build_typescript_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::TypeScriptTsx => build_tsx_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Json => build_json_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Markdown => build_markdown_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Bash => build_bash_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::C => build_c_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Cpp => build_cpp_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::CSharp => build_csharp_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Css => build_css_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Go => build_go_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Html => build_html_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Java => build_java_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Php => build_php_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Python => build_python_config(),
        #[cfg(feature = "code-highlight-official")]
        CodeLanguageKey::Ruby => build_ruby_config(),
        #[cfg(feature = "code-highlight-config")]
        CodeLanguageKey::Yaml => build_yaml_config(),
        #[cfg(feature = "code-highlight-config")]
        CodeLanguageKey::Toml => build_toml_config(),
        _ => None,
    }
}

#[cfg(all(feature = "code-highlight-core", feature = "code-highlight-official"))]
fn configure_highlights(
    language: tree_sitter::Language,
    name: &'static str,
    highlights_query: &str,
    injections_query: &str,
    locals_query: &str,
) -> Option<HighlightConfiguration> {
    let mut config = HighlightConfiguration::new(
        language,
        name,
        highlights_query,
        injections_query,
        locals_query,
    )
    .ok()?;
    config.configure(HIGHLIGHT_NAMES);
    Some(config)
}

#[cfg(all(feature = "code-highlight-core", feature = "code-highlight-official"))]
fn build_rust_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_rust::LANGUAGE.into(),
        "rust",
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY,
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_javascript_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_javascript::LANGUAGE.into(),
        "javascript",
        tree_sitter_javascript::HIGHLIGHT_QUERY,
        tree_sitter_javascript::INJECTIONS_QUERY,
        tree_sitter_javascript::LOCALS_QUERY,
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_jsx_config() -> Option<HighlightConfiguration> {
    let query = format!(
        "{}\n{}",
        tree_sitter_javascript::HIGHLIGHT_QUERY,
        tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
    );
    configure_highlights(
        tree_sitter_javascript::LANGUAGE.into(),
        "javascript",
        &query,
        tree_sitter_javascript::INJECTIONS_QUERY,
        tree_sitter_javascript::LOCALS_QUERY,
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_typescript_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "typescript",
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
        tree_sitter_typescript::LOCALS_QUERY,
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_tsx_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_typescript::LANGUAGE_TSX.into(),
        "tsx",
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
        tree_sitter_typescript::LOCALS_QUERY,
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_json_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_json::LANGUAGE.into(),
        "json",
        tree_sitter_json::HIGHLIGHTS_QUERY,
        "",
        "",
    )
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

#[cfg(feature = "code-highlight-official")]
fn build_markdown_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_md::LANGUAGE.into(),
        "markdown",
        MARKDOWN_HIGHLIGHT_QUERY,
        MARKDOWN_INJECTION_QUERY,
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_bash_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_bash::LANGUAGE.into(),
        "bash",
        tree_sitter_bash::HIGHLIGHT_QUERY,
        "",
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_c_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_c::LANGUAGE.into(),
        "c",
        tree_sitter_c::HIGHLIGHT_QUERY,
        "",
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_cpp_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_cpp::LANGUAGE.into(),
        "cpp",
        tree_sitter_cpp::HIGHLIGHT_QUERY,
        "",
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_csharp_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_c_sharp::LANGUAGE.into(),
        "c_sharp",
        tree_sitter_c_sharp::HIGHLIGHTS_QUERY,
        "",
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_css_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_css::LANGUAGE.into(),
        "css",
        tree_sitter_css::HIGHLIGHTS_QUERY,
        "",
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_go_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_go::LANGUAGE.into(),
        "go",
        tree_sitter_go::HIGHLIGHTS_QUERY,
        "",
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_html_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_html::LANGUAGE.into(),
        "html",
        tree_sitter_html::HIGHLIGHTS_QUERY,
        tree_sitter_html::INJECTIONS_QUERY,
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_java_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_java::LANGUAGE.into(),
        "java",
        tree_sitter_java::HIGHLIGHTS_QUERY,
        "",
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_php_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_php::LANGUAGE_PHP.into(),
        "php",
        tree_sitter_php::HIGHLIGHTS_QUERY,
        tree_sitter_php::INJECTIONS_QUERY,
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_python_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_python::LANGUAGE.into(),
        "python",
        tree_sitter_python::HIGHLIGHTS_QUERY,
        "",
        "",
    )
}

#[cfg(feature = "code-highlight-official")]
fn build_ruby_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_ruby::LANGUAGE.into(),
        "ruby",
        tree_sitter_ruby::HIGHLIGHTS_QUERY,
        "",
        tree_sitter_ruby::LOCALS_QUERY,
    )
}

#[cfg(all(feature = "code-highlight-core", feature = "code-highlight-config"))]
#[cfg(feature = "code-highlight-config")]
fn build_yaml_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_yaml::LANGUAGE.into(),
        "yaml",
        tree_sitter_yaml::HIGHLIGHTS_QUERY,
        "",
        "",
    )
}

#[cfg(all(feature = "code-highlight-core", feature = "code-highlight-config"))]
#[cfg(feature = "code-highlight-config")]
fn build_toml_config() -> Option<HighlightConfiguration> {
    configure_highlights(
        tree_sitter_toml::LANGUAGE.into(),
        "toml",
        tree_sitter_toml::HIGHLIGHTS_QUERY,
        "",
        "",
    )
}

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

    #[cfg(feature = "code-highlight-core")]
    {
        // Markdown runs two passes: a base pass for block structure and an
        // overlay pass with injections (inline markup, fenced code
        // languages), merged so heading colors show through between
        // emphasized runs.
        #[cfg(feature = "code-highlight-official")]
        if key == CodeLanguageKey::Markdown {
            return Some(CodeHighlightResult {
                language: key,
                spans: highlight_markdown(source),
            });
        }
        if let Some(config) = CODE_HIGHLIGHT_REGISTRY.config_for(key) {
            let spans = collect_highlight_spans(&config, source, &HashMap::new());
            return Some(CodeHighlightResult {
                language: key,
                spans,
            });
        }
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

/// Runs the highlighter over `source`, resolving injections against the
/// given configuration map (language name → configuration).
#[cfg(feature = "code-highlight-core")]
fn collect_highlight_spans(
    config: &HighlightConfiguration,
    source: &str,
    injections: &HashMap<String, Arc<HighlightConfiguration>>,
) -> Vec<CodeHighlightSpan> {
    let mut highlighter = Highlighter::new();
    let events = match highlighter.highlight(config, source.as_bytes(), None, |name| {
        injections.get(name).map(|arc| arc.as_ref())
    }) {
        Ok(events) => events,
        Err(_) => return Vec::new(),
    };

    let mut spans = Vec::new();
    let mut active = Vec::new();
    for event in events {
        let Ok(event) = event else {
            return Vec::new();
        };

        match event {
            HighlightEvent::Source { start, end } => {
                if let Some(class) = active.last().copied() {
                    push_highlight_span(&mut spans, start..end, class);
                }
            }
            HighlightEvent::HighlightStart(highlight) => {
                if let Some(class) = class_for_highlight(highlight) {
                    active.push(class);
                }
            }
            HighlightEvent::HighlightEnd => {
                active.pop();
            }
        }
    }

    spans
}

/// The `markdown_inline` grammar configuration, built once. Its capture
/// names share the global `HIGHLIGHT_NAMES` list, so highlight indices are
/// compatible across injected configurations.
#[cfg(feature = "code-highlight-official")]
fn markdown_inline_config() -> Option<Arc<HighlightConfiguration>> {
    static INLINE_CONFIG: LazyLock<Option<Arc<HighlightConfiguration>>> = LazyLock::new(|| {
        configure_highlights(
            tree_sitter_md::INLINE_LANGUAGE.into(),
            "markdown_inline",
            tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
            tree_sitter_md::INJECTION_QUERY_INLINE,
            "",
        )
        .map(Arc::new)
    });
    INLINE_CONFIG.clone()
}

/// Highlight Markdown with block structure and inline markup combined:
/// the base pass colors headings, list and quote markers, and fences; the
/// overlay pass adds emphasis, links, inline code, and fenced-code
/// languages through injections.
#[cfg(feature = "code-highlight-official")]
fn highlight_markdown(source: &str) -> Vec<CodeHighlightSpan> {
    let Some(config) = CODE_HIGHLIGHT_REGISTRY.config_for(CodeLanguageKey::Markdown) else {
        return Vec::new();
    };

    // Base pass without injections: heading color spans cover their full
    // inline content.
    let base = collect_highlight_spans(&config, source, &HashMap::new());

    // Overlay pass: inline markup plus fenced-code languages.
    let mut injections: HashMap<String, Arc<HighlightConfiguration>> = HashMap::new();
    if let Some(cfg) = markdown_inline_config() {
        injections.insert("markdown_inline".to_string(), cfg);
    }
    for name in ["html", "yaml", "toml"] {
        if let Some(key) = resolve_code_language_key(Some(name))
            && let Some(cfg) = CODE_HIGHLIGHT_REGISTRY.config_for(key)
        {
            injections.insert(name.to_string(), cfg);
        }
    }
    for lang in fence_languages(source) {
        if injections.contains_key(lang) {
            continue;
        }
        if let Some(key) = resolve_code_language_key(Some(lang))
            && let Some(cfg) = CODE_HIGHLIGHT_REGISTRY.config_for(key)
        {
            injections.insert(lang.to_string(), cfg);
        }
    }
    let overlay = collect_highlight_spans(&config, source, &injections);

    merge_span_layers(base, overlay)
}

/// Language tags of fenced code blocks (` ```rust ` → "rust"), driving the
/// dynamic injection lookup.
#[cfg(feature = "code-highlight-official")]
fn fence_languages(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let fence = line
                .strip_prefix("```")
                .or_else(|| line.strip_prefix("~~~"))?;
            let lang = fence.split_whitespace().next()?;
            (!lang.is_empty()).then_some(lang)
        })
        .collect()
}

/// Merges a base span layer with an overlay layer: overlay spans win where
/// they exist, base spans fill the gaps. Both layers are sorted by range
/// start and non-overlapping within themselves, so this is one linear
/// pass.
#[cfg(feature = "code-highlight-core")]
fn merge_span_layers(
    mut base: Vec<CodeHighlightSpan>,
    overlay: Vec<CodeHighlightSpan>,
) -> Vec<CodeHighlightSpan> {
    let mut merged = Vec::with_capacity(base.len() + overlay.len());
    let mut base_idx = 0usize;
    for span in overlay {
        // Base spans entirely before the overlay span.
        while base_idx < base.len() && base[base_idx].range.end <= span.range.start {
            merged.push(base[base_idx].clone());
            base_idx += 1;
        }
        // The head of the base span straddling the overlay start.
        if base_idx < base.len() && base[base_idx].range.start < span.range.start {
            let head = CodeHighlightSpan {
                range: base[base_idx].range.start..span.range.start,
                class: base[base_idx].class,
            };
            if head.range.start < head.range.end {
                merged.push(head);
            }
            base[base_idx].range.start = span.range.start;
        }
        // The overlay span itself.
        if span.range.start < span.range.end {
            merged.push(span.clone());
        }
        // Skip base spans fully covered by the overlay; trim the one
        // straddling the overlay end.
        while base_idx < base.len() && base[base_idx].range.start < span.range.end {
            if base[base_idx].range.end <= span.range.end {
                base_idx += 1;
            } else {
                base[base_idx].range.start = span.range.end;
                break;
            }
        }
    }
    // Base spans after the last overlay span.
    while base_idx < base.len() {
        merged.push(base[base_idx].clone());
        base_idx += 1;
    }
    merged
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

#[cfg(feature = "code-highlight-core")]
fn class_for_highlight(highlight: Highlight) -> Option<CodeHighlightClass> {
    let name = HIGHLIGHT_NAMES.get(highlight.0)?;
    Some(match *name {
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
