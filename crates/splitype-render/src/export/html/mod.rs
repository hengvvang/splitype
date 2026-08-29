//! HTML document generation for Markdown export.

pub mod document;
pub mod rewriter;
pub mod styles;

pub use document::{
    contains_tibetan_text, render_chromium_pdf_html_with_base_dir, render_html,
    render_html_with_base_dir,
};


#[cfg(test)]
mod tests {
    use super::{
        contains_tibetan_text, render_chromium_pdf_html_with_base_dir, render_html,
        render_html_with_base_dir,
    };
    use splitype_infra::theme::Theme;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn renders_complete_html_document_with_theme_css() {
        let html = render_html("# Title\n\ntext", &Theme::default_theme(), "Doc");

        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("<title>Doc</title>"));
        assert!(html.contains("<style>"));
        assert!(html.contains("--vlt-bg:"));
        assert!(html.contains("<main class=\"vlt-document\">"));
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<p>text</p>"));
    }
    #[test]
    fn detects_tibetan_text_for_document_language() {
        assert!(contains_tibetan_text("\u{0f56}\u{0f7c}\u{0f51}"));
        assert!(!contains_tibetan_text("Chinese text"));
    }

    #[test]
    fn exports_tibetan_with_language_and_font_fallbacks() {
        let markdown = concat!(
            "\u{0f56}\u{0f7c}\u{0f51}\u{0f0b}\u{0f61}\u{0f72}\u{0f42}",
            " ",
            "\u{0f56}\u{0f7c}\u{0f51}\u{0f0b}\u{0f61}\u{0f72}\u{0f42} "
        );
        let html = render_html(markdown, &Theme::default_theme(), "Doc");

        assert!(html.contains("<html lang=\"bo\">"));
        assert!(html.contains("\u{0f56}\u{0f7c}\u{0f51}"));
        assert!(html.contains("\u{0f61}\u{0f72}\u{0f42}"));
        assert!(html.contains("\"Noto Serif Tibetan\""));
        assert!(html.contains("\"Microsoft Himalaya\""));
    }
    #[test]
    fn emits_pdf_compatible_theme_css() {
        let html = render_html("# Title\n\ntext", &Theme::default_theme(), "Doc");

        assert!(!html.contains("hsla("));
        assert!(html.contains("color-scheme: dark;"));
        assert!(html.contains("--vlt-bg: rgba(25,25,25,1.000);"));
        assert!(html.contains("html { background-color: var(--vlt-bg); color: var(--vlt-text); }"));
        assert!(html.contains("background-color: var(--vlt-code-bg);"));
        assert!(html.contains("border: 1px solid;\n  border-color: var(--vlt-border);"));
        assert!(html.contains(
            "blockquote.markdown-alert-note { background-color: var(--vlt-callout-note-bg); border-color: var(--vlt-callout-note-border); }"
        ));
        assert!(!html.contains("background: var("));
        assert!(!html.contains("border-left-color:"));
    }

    #[test]
    fn light_theme_exports_light_color_scheme() {
        let html = render_html("# Title\n\ntext", &Theme::light_theme(), "Doc");

        assert!(html.contains("color-scheme: light;"));
        assert!(html.contains("--vlt-bg: rgba(255,255,255,1.000);"));
        assert!(!html.contains("color-scheme: dark;"));
    }

    #[test]
    fn chromium_pdf_light_theme_clears_print_container_frames() {
        let html = render_chromium_pdf_html_with_base_dir(
            "# Title\n\ntext",
            &Theme::light_theme(),
            "Doc",
            None,
        );

        assert!(html.contains("color-scheme: light;"));
        assert!(html.contains("background-color: var(--vlt-bg);"));
        assert!(html.contains("border: 0;"));
        assert!(html.contains("outline: 0;"));
        assert!(html.contains("box-shadow: none;"));
        assert!(!html.contains("color-scheme: dark;"));
    }

    #[test]
    fn enables_extended_markdown_features() {
        let markdown = "> [!NOTE]\n> body\n\n| A | B |\n| - | - |\n| 1 | 2 |\n\n- [x] done\n\n~~old~~\n\nhello[^a]\n\n[^a]: footnote";
        let html = render_html(markdown, &Theme::default_theme(), "Doc");

        assert!(html.contains("markdown-alert-note"));
        assert!(html.contains("<table>"));
        assert!(html.contains("checked"));
        assert!(html.contains("<del>old</del>"));
        assert!(html.contains("footnote"));
    }

    #[test]
    fn renders_splitype_comment_blocks_as_visible_escaped_text() {
        let markdown = "<!--\n<strong>not html</strong>\n-->";
        let html = render_html(markdown, &Theme::default_theme(), "Doc");

        assert!(html.contains("class=\"vlt-comment\""));
        assert!(html.contains("&lt;!--"));
        assert!(html.contains("&lt;strong&gt;not html&lt;/strong&gt;"));
        assert!(!html.contains("<!--\n<strong>not html</strong>\n-->"));
    }

    #[test]
    fn does_not_rewrite_comment_markers_inside_fenced_code() {
        let markdown = "```\n<!--\nnot a comment block\n-->\n```";
        let html = render_html(markdown, &Theme::default_theme(), "Doc");

        assert!(!html.contains("class=\"vlt-comment\""));
        assert!(html.contains("&lt;!--"));
        assert!(html.contains("not a comment block"));
    }

    #[test]
    fn escapes_risky_raw_html_blocks_for_export() {
        let html = render_html("<script>alert(1)</script>", &Theme::default_theme(), "Doc");

        assert!(html.contains("class=\"vlt-raw-html\""));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn escapes_risky_child_inside_safe_html_for_export() {
        let html = render_html(
            "<div>safe<script>alert(1)</script>tail</div>",
            &Theme::default_theme(),
            "Doc",
        );

        assert!(html.contains("<div>safe"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("tail</div>"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn sanitizes_safe_html_style_attributes_for_export() {
        let html = render_html(
            "<span style=\"color:blue; background-image:url(javascript:bad); background-color:#ff0; font-size:120%\">x</span>",
            &Theme::default_theme(),
            "Doc",
        );

        assert!(html.contains(
            "style=\"color: rgba(0,0,255,1.000); background-color: rgba(255,255,0,1.000); font-size: 120%;\""
        ));
        assert!(!html.contains("background-image"));
    }

    #[test]
    fn escapes_title_and_markdown_body_html() {
        let html = render_html("# A & B", &Theme::default_theme(), "A & <B>");

        assert!(html.contains("<title>A &amp; &lt;B&gt;</title>"));
        assert!(html.contains("<h1>A &amp; B</h1>"));
    }

    #[test]
    fn exports_display_math_as_svg() {
        let html = render_html("$$\n\\frac{1}{2}\n$$", &Theme::default_theme(), "Doc");

        assert!(html.contains("class=\"vlt-math\""));
        assert!(html.contains("<svg"));
        assert!(!html.contains("$$\n\\frac{1}{2}\n$$"));
    }

    #[test]
    fn exports_mermaid_block_as_svg() {
        let html = render_html(
            "```mermaid\nflowchart LR\nA --> B\n```",
            &Theme::default_theme(),
            "Doc",
        );

        assert!(html.contains("class=\"vlt-mermaid\""));
        assert!(html.contains("<img alt=\"Mermaid diagram\""));
        assert!(html.contains("data:image/svg+xml;base64,"));
        assert!(!html.contains("```mermaid\nflowchart LR\nA --&gt; B\n```"));
    }

    #[test]
    fn exports_local_image_as_data_uri_when_base_dir_is_available() {
        let root = std::env::temp_dir().join(format!("splitype-html-export-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp export dir");
        fs::write(
            root.join("diagram.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1 1\"></svg>",
        )
        .expect("write local image");

        let html = render_html_with_base_dir(
            "![diagram](diagram.svg)",
            &Theme::default_theme(),
            "Doc",
            Some(&root),
        );
        let _ = fs::remove_dir_all(&root);

        assert!(html.contains("data:image/svg+xml;base64,"));
        assert!(!html.contains("src=\"diagram.svg\""));
    }

    #[test]
    fn exports_standalone_html_image_with_sanitized_zoom() {
        let root = std::env::temp_dir().join(format!("splitype-html-export-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp export dir");
        fs::write(root.join("diagram.png"), [137, 80, 78, 71]).expect("write local image");

        let html = render_html_with_base_dir(
            "<img src=\"diagram.png\" alt=\"diagram\" style=\"color:red; zoom:80%; width:10px\" />",
            &Theme::default_theme(),
            "Doc",
            Some(&root),
        );
        let _ = fs::remove_dir_all(&root);

        assert!(html.contains("<img src=\"data:image/png;base64,"));
        assert!(html.contains("alt=\"diagram\""));
        assert!(html.contains("style=\"zoom: 80%;\""));
        assert!(!html.contains("color:red"));
        assert!(!html.contains("width:10px"));
    }

    #[test]
    fn export_keeps_missing_local_image_path() {
        let root = std::env::temp_dir().join(format!("splitype-html-export-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create temp export dir");

        let html = render_html_with_base_dir(
            "![diagram](missing.png)",
            &Theme::default_theme(),
            "Doc",
            Some(&root),
        );
        let _ = fs::remove_dir_all(&root);

        assert!(html.contains("src=\"missing.png\""));
        assert!(!html.contains("data:image/png;base64,"));
    }

    #[test]
    fn exports_inline_math_as_svg() {
        let html = render_html("before $x^2$ after", &Theme::default_theme(), "Doc");

        assert!(html.contains("class=\"vlt-inline-math\""));
        assert!(html.contains("<svg"));
        assert!(!html.contains("$x^2$"));
        assert!(html.contains("before"));
        assert!(html.contains("after"));
    }

    #[test]
    fn export_inline_math_ignores_code_and_escaped_delimiters() {
        let html = render_html("`$x$` and \\$y$", &Theme::default_theme(), "Doc");

        assert!(!html.contains("class=\"vlt-inline-math\""));
        assert!(html.contains("$x$"));
        assert!(html.contains("$y$"));
    }

    #[test]
    fn exports_superscript_and_subscript_as_html_tags() {
        let html = render_html("x^2^ and H~2~O", &Theme::default_theme(), "Doc");

        assert!(html.contains("x<sup>2</sup>"));
        assert!(html.contains("H<sub>2</sub>O"));
    }

    #[test]
    fn export_script_rewrite_ignores_code_escaped_and_strikethrough() {
        let html = render_html(
            "`x^2^ H~2~O` \\^2^ \\~2~ ~~old~~",
            &Theme::default_theme(),
            "Doc",
        );

        assert!(!html.contains("<sup>2</sup>"));
        assert!(!html.contains("<sub>2</sub>"));
        assert!(html.contains("<code>x^2^ H~2~O</code>"));
        assert!(html.contains("^2^"));
        assert!(html.contains("~2~"));
        assert!(html.contains("<del>old</del>"));
    }

    #[test]
    fn invalid_display_math_exports_escaped_raw_markdown() {
        let html = render_html("$$\n\\frac{a}\n$$", &Theme::default_theme(), "Doc");

        assert!(html.contains("class=\"vlt-math-error\""));
        assert!(html.contains("$$\n\\frac{a}\n$$"));
        assert!(!html.contains("class=\"vlt-math\"><svg"));
    }

    #[test]
    fn invalid_mermaid_exports_escaped_raw_markdown() {
        let html = render_html(
            "```mermaid\nnot a real mermaid diagram ::::\n```",
            &Theme::default_theme(),
            "Doc",
        );

        assert!(html.contains("class=\"vlt-mermaid-error\""));
        assert!(html.contains("not a real mermaid diagram ::::"));
        assert!(!html.contains("data:image/svg+xml;base64,"));
    }
}
