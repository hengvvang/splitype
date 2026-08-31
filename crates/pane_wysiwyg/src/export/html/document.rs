//! Assembly of the final HTML document container and browser body rendering.

use std::path::Path;

use pulldown_cmark::{Options, Parser, html};

use crate::export::html::rewriter::{
    rewrite_display_math_blocks, rewrite_inline_math, rewrite_local_image_event,
    rewrite_mermaid_blocks, rewrite_unsafe_html_blocks, rewrite_visible_comment_blocks,
};
use crate::export::html::styles::{chromium_pdf_theme_css, escape_html, theme_css};
use theme::Theme;

/// Builds a full HTML document with embedded CSS derived from the active theme.
pub fn render_html(markdown: &str, theme: &Theme, title: &str) -> String {
    render_html_with_base_dir(markdown, theme, title, None)
}

/// Builds export HTML and resolves local Markdown image paths relative to the source document.
pub fn render_html_with_base_dir(
    markdown: &str,
    theme: &Theme,
    title: &str,
    base_dir: Option<&Path>,
) -> String {
    render_html_document(markdown, theme, title, base_dir, &theme_css(theme))
}

/// Builds HTML tailored for Chromium's print-to-PDF pipeline.
pub fn render_chromium_pdf_html_with_base_dir(
    markdown: &str,
    theme: &Theme,
    title: &str,
    base_dir: Option<&Path>,
) -> String {
    render_html_document(
        markdown,
        theme,
        title,
        base_dir,
        &chromium_pdf_theme_css(theme),
    )
}

fn render_html_document(
    markdown: &str,
    theme: &Theme,
    title: &str,
    base_dir: Option<&Path>,
    css: &str,
) -> String {
    let document_lang = if contains_tibetan_text(markdown) || contains_tibetan_text(title) {
        "bo"
    } else {
        "en"
    };
    let body = render_browser_html_body(markdown, theme, base_dir);

    format!(
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>\n{}\n</style>\n</head>\n<body>\n<main class=\"vlt-document\">\n{}</main>\n</body>\n</html>\n",
        document_lang,
        escape_html(title),
        css,
        body,
    )
}

fn markdown_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_GFM);
    options
}

fn render_browser_html_body(markdown: &str, theme: &Theme, base_dir: Option<&Path>) -> String {
    let rewritten = rewrite_visible_comment_blocks(markdown);
    let rewritten = rewrite_unsafe_html_blocks(&rewritten, base_dir);
    let rewritten = rewrite_display_math_blocks(&rewritten, theme);
    let rewritten = rewrite_inline_math(&rewritten, theme);
    let rewritten = rewrite_mermaid_blocks(&rewritten);
    let parser = Parser::new_ext(&rewritten, markdown_options())
        .map(|event| rewrite_local_image_event(event, base_dir));
    let mut body = String::new();
    html::push_html(&mut body, parser);
    body
}

pub fn contains_tibetan_text(text: &str) -> bool {
    text.chars()
        .any(|ch| ('\u{0f00}'..='\u{0fff}').contains(&ch))
}
