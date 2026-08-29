#[cfg(test)]
mod tests {
    use crate::export::html::render_html;
    use crate::plugins::code_highlight::highlight::{
        CodeLanguageKey, highlight_code_block, resolve_code_language_key,
    };
    use crate::plugins::latex_render::render_latex_to_svg;
    use crate::plugins::mermaid_render::render_mermaid_to_svg;
    use gpui::hsla;
    use splitype_infra::theme::Theme;

    #[test]
    fn test_code_language_resolution() {
        assert_eq!(resolve_code_language_key(Some("rs")), Some(CodeLanguageKey::Rust));
        assert_eq!(resolve_code_language_key(Some("rust")), Some(CodeLanguageKey::Rust));
        assert_eq!(resolve_code_language_key(Some("python")), Some(CodeLanguageKey::Python));
        assert_eq!(resolve_code_language_key(Some("mermaid")), Some(CodeLanguageKey::Mermaid));
    }

    #[test]
    fn test_syntax_highlight_block() {
        let code = "fn main() {\n    let x = 42;\n}";
        let result = highlight_code_block(Some("rust"), code);
        assert!(result.is_some());
        let res = result.unwrap();
        assert_eq!(res.language, CodeLanguageKey::Rust);
        assert!(!res.spans.is_empty());
    }

    #[test]
    fn test_html_export_generation() {
        let theme = Theme::default_theme();
        let markdown = "# Title\n\nSome **bold** text.";
        let html = render_html(markdown, &theme, "Test Document");
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_latex_to_svg_rendering() {
        let latex = "E = mc^2";
        let color = hsla(0.0, 0.0, 1.0, 1.0);
        let svg_result = render_latex_to_svg(latex, color, 16.0);
        assert!(svg_result.is_ok());
        let svg = svg_result.unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_mermaid_to_svg_rendering() {
        let diagram = "graph TD\n    A --> B";
        let svg_result = render_mermaid_to_svg(diagram);
        assert!(svg_result.is_ok());
        let svg = svg_result.unwrap();
        assert!(svg.contains("<svg"));
    }
}
