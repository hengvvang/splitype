//! Mermaid SVG rendering helpers and caching.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, anyhow};
use directories::ProjectDirs;

use markdown_parser::block::mermaid::MermaidSource;

const SIMPLE_MERMAID_LINE_LIMIT: usize = 8;
const MERMAID_COMPLEX_TARGET_WIDTH_RATIO: f32 = 0.9;
const MERMAID_MAX_VIEWPORT_WIDTH_RATIO: f32 = 1.15;
const MERMAID_SCALE_PER_EXTRA_LINE: f32 = 0.035;
const MERMAID_MAX_SCALE: f32 = 1.75;

/// Result of rendering a Mermaid diagram into an SVG cache file.
#[derive(Clone, Debug, PartialEq)]
pub struct MermaidSvgRender {
    /// Path to the SVG file consumed by GPUI's image element.
    pub path: PathBuf,
    /// SVG document content, used by export paths.
    pub svg: String,
    /// Concrete display width encoded into the cached SVG.
    pub display_width: f32,
    /// Concrete display height encoded into the cached SVG.
    pub display_height: f32,
    /// Scale applied to the renderer's intrinsic SVG size for editor display.
    pub display_scale: f32,
}

/// Concrete dimensions encoded into a display SVG.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MermaidSvgSize {
    pub width: f32,
    pub height: f32,
}

/// Render Mermaid source into a cached SVG sized for editor display.
pub fn render_mermaid_svg_for_display(
    source: &MermaidSource,
    available_width: f32,
    viewport_width: f32,
) -> anyhow::Result<MermaidSvgRender> {
    render_mermaid_svg_for_display_with(source, available_width, viewport_width, render_mermaid_raw)
}

fn render_mermaid_svg_for_display_with(
    source: &MermaidSource,
    available_width: f32,
    viewport_width: f32,
    renderer: MermaidRenderer,
) -> anyhow::Result<MermaidSvgRender> {
    let base_key = mermaid_cache_key(&source.body);
    let base_path = mermaid_base_cache_path(&base_key)?;
    let base_svg = render_mermaid_to_svg_cached_with(&source.body, &base_path, renderer)?;
    let intrinsic = mermaid_svg_intrinsic_size(&base_svg)?;
    let scale = mermaid_display_scale(
        &source.body,
        intrinsic.width,
        intrinsic.height,
        available_width,
        viewport_width,
    );

    let display_key = mermaid_display_cache_key(&source.body, scale);
    let display_path = mermaid_display_cache_path(&display_key)?;
    if display_path.exists() {
        let svg = fs::read_to_string(&display_path).with_context(|| {
            format!(
                "failed to read Mermaid display SVG cache '{}'",
                display_path.display()
            )
        })?;
        let size = mermaid_svg_intrinsic_size(&svg)?;
        return Ok(MermaidSvgRender {
            path: display_path,
            svg,
            display_width: size.width,
            display_height: size.height,
            display_scale: scale,
        });
    }

    let (svg, size) = scale_mermaid_svg_for_display(&base_svg, scale)?;
    fs::write(&display_path, &svg).with_context(|| {
        format!(
            "failed to write Mermaid display SVG cache '{}'",
            display_path.display()
        )
    })?;
    Ok(MermaidSvgRender {
        path: display_path,
        svg,
        display_width: size.width,
        display_height: size.height,
        display_scale: scale,
    })
}

/// Render a Mermaid diagram body into cached SVG text.
pub fn render_mermaid_to_svg(source: &str) -> anyhow::Result<String> {
    let key = mermaid_cache_key(source);
    let path = mermaid_base_cache_path(&key)?;
    render_mermaid_to_svg_cached_with(source, &path, render_mermaid_raw)
}

type MermaidRenderer = fn(&str) -> anyhow::Result<String>;

fn render_mermaid_to_svg_cached_with(
    source: &str,
    path: &Path,
    renderer: MermaidRenderer,
) -> anyhow::Result<String> {
    if path.exists() {
        return fs::read_to_string(path).with_context(|| {
            format!("failed to read Mermaid base SVG cache '{}'", path.display())
        });
    }

    let svg = renderer(source)?;
    fs::write(path, &svg).with_context(|| {
        format!(
            "failed to write Mermaid base SVG cache '{}'",
            path.display()
        )
    })?;
    Ok(svg)
}

const CROSS_PLATFORM_MERMAID_FONT_STACK: &str = r#""Segoe UI", "Microsoft YaHei", "PingFang SC", "Hiragino Sans GB", "Noto Sans CJK SC", "WenQuanYi Micro Hei", sans-serif"#;

fn inject_cjk_font_family(svg: &str) -> String {
    svg.replace(
        "\"trebuchet ms\", verdana, arial, sans-serif",
        CROSS_PLATFORM_MERMAID_FONT_STACK,
    )
    .replace(
        "'trebuchet ms', verdana, arial, sans-serif",
        CROSS_PLATFORM_MERMAID_FONT_STACK,
    )
    .replace(
        "\"Segoe UI\", sans-serif",
        CROSS_PLATFORM_MERMAID_FONT_STACK,
    )
    .replace("'Segoe UI', sans-serif", CROSS_PLATFORM_MERMAID_FONT_STACK)
}

fn render_mermaid_raw(source: &str) -> anyhow::Result<String> {
    if !looks_like_supported_mermaid_source(source) {
        return Err(anyhow::anyhow!("unsupported Mermaid diagram"));
    }
    let source_owned = source.to_string();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        mermaid_rs_renderer::render(&source_owned)
    }));
    let svg = match result {
        Ok(Ok(svg)) => svg,
        Ok(Err(err)) => return Err(anyhow::anyhow!("{err}")),
        Err(_) => return Err(anyhow::anyhow!("Mermaid renderer internal panic")),
    };
    if svg.contains("class=\"error-text\"") || svg.contains("Syntax error in text") {
        return Err(anyhow::anyhow!("Mermaid syntax error"));
    }
    Ok(inject_cjk_font_family(&svg))
}

/// Stable cache key for Mermaid content.
pub fn mermaid_cache_key(source: &str) -> String {
    format!("{:016x}", mermaid_content_fingerprint(source))
}

/// Stable 64-bit fingerprint for Mermaid diagram content.
pub fn mermaid_content_fingerprint(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Stable cache key for editor display SVG content and scale.
pub(crate) fn mermaid_display_cache_key(source: &str, scale: f32) -> String {
    let mut hasher = DefaultHasher::new();
    mermaid_cache_key(source).hash(&mut hasher);
    scale.max(0.1).to_bits().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Counts diagram lines that materially contribute to rendered complexity.
pub(crate) fn semantic_mermaid_line_count(source: &str) -> usize {
    let mut in_frontmatter = false;
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }
            if trimmed == "---" {
                in_frontmatter = !in_frontmatter;
                return false;
            }
            !(in_frontmatter || trimmed.starts_with("%%"))
        })
        .count()
}

/// Display scale used by the editor for rendered Mermaid diagrams.
pub(crate) fn mermaid_display_scale(
    source: &str,
    intrinsic_width: f32,
    intrinsic_height: f32,
    available_width: f32,
    viewport_width: f32,
) -> f32 {
    let line_count = semantic_mermaid_line_count(source);
    if line_count <= SIMPLE_MERMAID_LINE_LIMIT {
        return 1.0;
    }

    let intrinsic_width = intrinsic_width.max(1.0);
    let intrinsic_height = intrinsic_height.max(1.0);
    let available_width = available_width.max(1.0);
    let viewport_width = viewport_width.max(available_width);
    let extra_lines = line_count.saturating_sub(SIMPLE_MERMAID_LINE_LIMIT) as f32;

    let complexity_scale = (1.0 + extra_lines * MERMAID_SCALE_PER_EXTRA_LINE)
        .max(1.0)
        .min(MERMAID_MAX_SCALE);
    let target_column_width = available_width * MERMAID_COMPLEX_TARGET_WIDTH_RATIO;
    let column_fit_scale = if intrinsic_width < target_column_width {
        target_column_width / intrinsic_width
    } else {
        1.0
    };
    let viewport_limit_scale =
        (viewport_width * MERMAID_MAX_VIEWPORT_WIDTH_RATIO / intrinsic_width).max(1.0);
    let height_sanity_scale =
        (viewport_width * MERMAID_MAX_VIEWPORT_WIDTH_RATIO / intrinsic_height).max(1.0);

    let raw_scale = complexity_scale
        .max(column_fit_scale)
        .min(viewport_limit_scale)
        .min(height_sanity_scale)
        .min(MERMAID_MAX_SCALE)
        .max(1.0);
    ((raw_scale * 20.0).round() / 20.0).max(1.0)
}

/// Rewrites the root SVG element so GPUI sees the intended intrinsic size.
pub(crate) fn scale_mermaid_svg_for_display(
    svg: &str,
    scale: f32,
) -> anyhow::Result<(String, MermaidSvgSize)> {
    let scale = scale.max(0.1);
    let (start, end) = svg_root_tag_range(svg)?;
    let root_tag = &svg[start..end];
    let base_size = svg_root_size(root_tag)?;
    let size = MermaidSvgSize {
        width: (base_size.width * scale).max(1.0),
        height: (base_size.height * scale).max(1.0),
    };
    let rewritten_root = rewrite_svg_root_tag(root_tag, size)?;
    let mut rewritten = String::with_capacity(svg.len() + 48);
    rewritten.push_str(&svg[..start]);
    rewritten.push_str(&rewritten_root);
    rewritten.push_str(&svg[end..]);
    Ok((rewritten, size))
}

fn mermaid_svg_intrinsic_size(svg: &str) -> anyhow::Result<MermaidSvgSize> {
    let (start, end) = svg_root_tag_range(svg)?;
    svg_root_size(&svg[start..end])
}

fn svg_root_tag_range(svg: &str) -> anyhow::Result<(usize, usize)> {
    let start = svg
        .find("<svg")
        .ok_or_else(|| anyhow!("Mermaid renderer output did not contain an SVG root"))?;
    let bytes = svg.as_bytes();
    let mut quote = None;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if byte == active_quote {
                quote = None;
            }
        } else if byte == b'"' || byte == b'\'' {
            quote = Some(byte);
        } else if byte == b'>' {
            return Ok((start, index + 1));
        }
        index += 1;
    }
    Err(anyhow!(
        "Mermaid renderer output had an unterminated SVG root tag"
    ))
}

fn svg_root_size(root_tag: &str) -> anyhow::Result<MermaidSvgSize> {
    if let Some(view_box) = svg_root_attr(root_tag, "viewBox")
        && let Some(size) = parse_view_box_size(&view_box)
    {
        return Ok(size);
    }

    let width = svg_root_attr(root_tag, "width")
        .and_then(|value| parse_svg_length(&value))
        .ok_or_else(|| anyhow!("Mermaid SVG root did not expose a usable width"))?;
    let height = svg_root_attr(root_tag, "height")
        .and_then(|value| parse_svg_length(&value))
        .ok_or_else(|| anyhow!("Mermaid SVG root did not expose a usable height"))?;
    Ok(MermaidSvgSize { width, height })
}

fn parse_view_box_size(view_box: &str) -> Option<MermaidSvgSize> {
    let values = view_box
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    (values.len() == 4 && values[2].is_finite() && values[3].is_finite()).then_some(
        MermaidSvgSize {
            width: values[2].max(1.0),
            height: values[3].max(1.0),
        },
    )
}

fn parse_svg_length(value: &str) -> Option<f32> {
    let value = value.trim();
    let end = value
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E'))
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    let parsed = value[..end].parse::<f32>().ok()?;
    (parsed.is_finite() && parsed > 0.0).then_some(parsed)
}

fn svg_root_attr(root_tag: &str, attr_name: &str) -> Option<String> {
    svg_root_attrs(root_tag)
        .into_iter()
        .find(|attr| attr.name.eq_ignore_ascii_case(attr_name))
        .and_then(|attr| attr.value)
}

fn rewrite_svg_root_tag(root_tag: &str, size: MermaidSvgSize) -> anyhow::Result<String> {
    let attrs = svg_root_attrs(root_tag)
        .into_iter()
        .filter(|attr| {
            !["width", "height", "style"]
                .iter()
                .any(|name| attr.name.eq_ignore_ascii_case(name))
        })
        .map(|attr| attr.raw)
        .collect::<Vec<_>>();

    let mut rewritten = String::from("<svg");
    for attr in attrs {
        rewritten.push(' ');
        rewritten.push_str(attr.trim());
    }
    rewritten.push_str(&format!(
        " width=\"{:.3}\" height=\"{:.3}\">",
        size.width, size.height
    ));
    Ok(rewritten)
}

#[derive(Debug)]
struct SvgRootAttr {
    name: String,
    value: Option<String>,
    raw: String,
}

fn svg_root_attrs(root_tag: &str) -> Vec<SvgRootAttr> {
    let Some(mut index) = root_tag.find("<svg").map(|index| index + "<svg".len()) else {
        return Vec::new();
    };
    let end = root_tag.rfind('>').unwrap_or(root_tag.len());
    let bytes = root_tag.as_bytes();
    let mut attrs = Vec::new();

    while index < end {
        while index < end && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= end || bytes[index] == b'/' {
            break;
        }

        let attr_start = index;
        while index < end
            && !bytes[index].is_ascii_whitespace()
            && bytes[index] != b'='
            && bytes[index] != b'/'
        {
            index += 1;
        }
        let name = root_tag[attr_start..index].to_string();
        if name.is_empty() {
            break;
        }

        while index < end && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        let mut value = None;
        if index < end && bytes[index] == b'=' {
            index += 1;
            while index < end && bytes[index].is_ascii_whitespace() {
                index += 1;
            }

            if index < end && (bytes[index] == b'"' || bytes[index] == b'\'') {
                let quote = bytes[index];
                index += 1;
                let value_start = index;
                while index < end && bytes[index] != quote {
                    index += 1;
                }
                value = Some(root_tag[value_start..index].to_string());
                if index < end {
                    index += 1;
                }
            } else {
                let value_start = index;
                while index < end && !bytes[index].is_ascii_whitespace() && bytes[index] != b'/' {
                    index += 1;
                }
                value = Some(root_tag[value_start..index].to_string());
            }
        }

        let raw = root_tag[attr_start..index].trim().to_string();
        attrs.push(SvgRootAttr { name, value, raw });
    }

    attrs
}

fn mermaid_cache_dir() -> anyhow::Result<PathBuf> {
    let root = ProjectDirs::from("com", "hengvvang", "splitype")
        .map(|dirs| dirs.cache_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("splitype"));
    let dir = root.join("mermaid-svg");
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create Mermaid SVG cache '{}'", dir.display()))?;
    Ok(dir)
}

fn mermaid_base_cache_path(key: &str) -> anyhow::Result<PathBuf> {
    mermaid_cache_file_path("base", key)
}

fn mermaid_display_cache_path(key: &str) -> anyhow::Result<PathBuf> {
    mermaid_cache_file_path("display", key)
}

fn mermaid_cache_file_path(kind: &str, key: &str) -> anyhow::Result<PathBuf> {
    let dir = mermaid_cache_dir()?.join(kind);
    fs::create_dir_all(&dir).with_context(|| {
        format!(
            "failed to create Mermaid {kind} SVG cache '{}'",
            dir.display()
        )
    })?;
    Ok(dir.join(format!("{key}.svg")))
}

fn looks_like_supported_mermaid_source(source: &str) -> bool {
    let mut in_frontmatter = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter || trimmed.starts_with("%%") {
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        return [
            "sequencediagram",
            "classdiagram",
            "statediagram",
            "erdiagram",
            "pie",
            "mindmap",
            "journey",
            "timeline",
            "gantt",
            "requirementdiagram",
            "gitgraph",
            "c4",
            "sankey",
            "quadrantchart",
            "zenuml",
            "block",
            "packet",
            "kanban",
            "architecture",
            "radar",
            "treemap",
            "xychart",
            "flowchart",
            "graph",
        ]
        .iter()
        .any(|prefix| lower.starts_with(prefix));
    }
    false
}
