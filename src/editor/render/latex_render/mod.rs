//! RaTeX SVG rendering helpers for LaTeX math.
//!
//! Display-math source parsing lives in `model::syntax::math`; this module
//! only owns the SVG rendering pipeline and its cache.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context as _, anyhow};
use directories::ProjectDirs;
use gpui::{Hsla, Rgba};

use crate::model::syntax::math::DisplayMathSource;

/// Upper bound on formulas retained in the in-memory SVG cache. Content
/// is addresses by hash, so eviction is safe: the disk cache still holds
/// the SVG and a miss just re-reads it.
const LATEX_SVG_MEMORY_CACHE_CAP: usize = 256;

/// Process-wide in-memory SVG cache shared by every render call site
/// (editor, preview, export). Without it, each frame re-runs the full
/// ratex parse + layout + glyph-embedding pipeline for every formula.
static LATEX_SVG_MEMORY_CACHE: OnceLock<Mutex<HashMap<u64, Arc<LatexSvgRender>>>> = OnceLock::new();

fn latex_svg_memory_cache() -> &'static Mutex<HashMap<u64, Arc<LatexSvgRender>>> {
    LATEX_SVG_MEMORY_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

const DISPLAY_MATH_SCALE: f32 = 1.25;
const INLINE_MATH_SCALE: f32 = 1.12;

/// Result of rendering display math into an SVG cache file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LatexSvgRender {
    /// Path to the SVG file consumed by GPUI's image element.
    pub(crate) path: PathBuf,
    /// SVG document content, used by export paths.
    pub(crate) svg: String,
}

/// Display font size used for rendered display-math blocks.
pub(crate) fn display_math_font_size(base_font_size: f32) -> f32 {
    base_font_size * DISPLAY_MATH_SCALE
}

/// Display font size used for rendered inline math.
pub(crate) fn inline_math_font_size(base_font_size: f32) -> f32 {
    base_font_size * INLINE_MATH_SCALE
}

/// Render a display-math source into a cached SVG file.
pub(crate) fn render_display_math_svg(
    source: &DisplayMathSource,
    text_color: Hsla,
    font_size: f32,
) -> anyhow::Result<Arc<LatexSvgRender>> {
    render_latex_svg_to_cache(&source.body, text_color, font_size)
}

/// Render an inline LaTeX body into a cached SVG file.
pub(crate) fn render_inline_math_svg(
    latex: &str,
    text_color: Hsla,
    font_size: f32,
) -> anyhow::Result<Arc<LatexSvgRender>> {
    render_latex_svg_to_cache(latex, text_color, font_size)
}

/// Resolve a formula to a cached SVG, rendering only on a total miss.
///
/// The lookup order is in-memory hash cache -> disk cache file -> render.
/// The previous implementation rendered first and only used the cache to
/// skip the disk write, so every frame re-ran the full ratex pipeline for
/// every formula in the document.
fn render_latex_svg_to_cache(
    latex: &str,
    text_color: Hsla,
    font_size: f32,
) -> anyhow::Result<Arc<LatexSvgRender>> {
    let key = latex_cache_fingerprint(latex, text_color, font_size);
    if let Some(cached) = latex_svg_memory_cache().lock().unwrap().get(&key) {
        return Ok(cached.clone());
    }

    let path = latex_cache_dir()?.join(format!("{key:016x}.svg"));
    let svg = if path.exists() {
        fs::read_to_string(&path)
            .with_context(|| format!("failed to read LaTeX SVG cache '{}'", path.display()))?
    } else {
        let svg = render_latex_to_svg(latex, text_color, font_size)?;
        fs::write(&path, &svg)
            .with_context(|| format!("failed to write LaTeX SVG cache '{}'", path.display()))?;
        svg
    };

    let rendered = Arc::new(LatexSvgRender { path, svg });
    let mut cache = latex_svg_memory_cache().lock().unwrap();
    if cache.len() >= LATEX_SVG_MEMORY_CACHE_CAP {
        cache.clear();
    }
    cache.insert(key, rendered.clone());
    Ok(rendered)
}

/// Render a LaTeX expression into self-contained SVG text.
pub(crate) fn render_latex_to_svg(
    latex: &str,
    text_color: Hsla,
    font_size: f32,
) -> anyhow::Result<String> {
    let parsed = ratex_parser::parse(latex).map_err(|err| anyhow!("{err}"))?;
    let layout = ratex_layout::layout(&parsed, &ratex_layout::LayoutOptions::default());
    let display_list = ratex_layout::to_display_list(&layout);
    let mut svg = ratex_svg::render_to_svg(
        &display_list,
        &ratex_svg::SvgOptions {
            font_size: f64::from(font_size.max(1.0)),
            padding: f64::from((font_size * 0.35).max(4.0)),
            embed_glyphs: true,
            ..ratex_svg::SvgOptions::default()
        },
    );
    svg = recolor_default_black(&svg, &svg_color(text_color));
    Ok(svg)
}

/// Stable 64-bit fingerprint for formula content and visual parameters.
pub(crate) fn latex_cache_fingerprint(latex: &str, text_color: Hsla, font_size: f32) -> u64 {
    let mut hasher = DefaultHasher::new();
    latex.hash(&mut hasher);
    svg_color(text_color).hash(&mut hasher);
    font_size.to_bits().hash(&mut hasher);
    hasher.finish()
}

fn latex_cache_dir() -> anyhow::Result<PathBuf> {
    let root = ProjectDirs::from("com", "hengvvang", "splitype")
        .map(|dirs| dirs.cache_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("splitype"));
    let dir = root.join("latex-svg");
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create LaTeX SVG cache '{}'", dir.display()))?;
    Ok(dir)
}

fn svg_color(color: Hsla) -> String {
    let color = Rgba::from(color);
    format!(
        "rgba({},{},{},{})",
        color_channel(color.r),
        color_channel(color.g),
        color_channel(color.b),
        trim_float(f64::from(color.a.clamp(0.0, 1.0)))
    )
}

fn color_channel(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn trim_float(value: f64) -> String {
    let formatted = format!("{value:.3}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn recolor_default_black(svg: &str, color: &str) -> String {
    svg.replace("rgba(0,0,0,1)", color)
        .replace("rgba(0, 0, 0, 1)", color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::syntax::math::parse_display_math_source;
    use gpui::rgba;

    #[test]
    fn parses_single_line_display_math() {
        let parsed = parse_display_math_source("$$x^2$$").expect("display math");
        assert_eq!(parsed.body, "x^2");
        assert_eq!(parsed.raw, "$$x^2$$");
    }

    #[test]
    fn parses_multiline_display_math() {
        let parsed = parse_display_math_source("$$\n\\int_0^1 x^2 dx\n$$").expect("display math");
        assert_eq!(parsed.body, "\\int_0^1 x^2 dx");
    }

    #[test]
    fn rejects_unclosed_display_math() {
        assert!(parse_display_math_source("$$\n\\frac{1}{2}").is_none());
    }

    #[test]
    fn cache_key_changes_with_theme_inputs() {
        let first = latex_cache_fingerprint("\\frac{1}{2}", Hsla::from(rgba(0xffffffff)), 18.0);
        let second = latex_cache_fingerprint("\\frac{1}{2}", Hsla::from(rgba(0x000000ff)), 18.0);
        assert_ne!(first, second);
    }

    #[test]
    fn display_math_font_size_scales_base_text_size() {
        assert_eq!(display_math_font_size(20.0), 25.0);
    }

    #[test]
    fn inline_math_font_size_scales_base_text_size() {
        assert!((inline_math_font_size(20.0) - 22.4).abs() < 0.001);
    }

    #[test]
    fn renders_basic_formula_svg() {
        let svg =
            render_latex_to_svg("\\frac{1}{2}", Hsla::from(rgba(0xffffffff)), 18.0).expect("svg");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn invalid_latex_returns_error() {
        assert!(render_latex_to_svg("\\frac{a}", Hsla::from(rgba(0xffffffff)), 18.0).is_err());
    }
}
