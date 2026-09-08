//! Breadcrumb bar for editor panes — displays the file path and outline toggle.

use gpui::prelude::FluentBuilder;
use gpui::*;
use std::path::{Path, PathBuf};
use theme::Theme;

/// Strips Windows verbatim prefix (`\\?\` or `\\?\UNC\`) if present.
pub fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", rest))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

/// Formats a file path relative to the project/workspace root directory,
/// ensuring no `\\?\` prefix appears, normalizes path separators to `/`,
/// and prefixes with the root folder name (e.g. `splitype/README.md`).
pub fn format_root_relative_path(path: &Path) -> String {
    let clean_path = strip_verbatim_prefix(path);

    if let Ok(cwd) = std::env::current_dir() {
        let clean_cwd = strip_verbatim_prefix(&cwd);
        if let Ok(rel) = clean_path.strip_prefix(&clean_cwd) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let root_name = clean_cwd
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if rel_str.is_empty() {
                return root_name;
            } else if !root_name.is_empty() {
                return format!("{}/{}", root_name, rel_str);
            } else {
                return rel_str;
            }
        }
    }

    // If not directly under cwd, check ancestors for project root markers (.git or Cargo.toml)
    let mut current = clean_path.parent();
    while let Some(dir) = current {
        if dir.join(".git").exists() || dir.join("Cargo.toml").exists() {
            if let Ok(rel) = clean_path.strip_prefix(dir) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                let root_name = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if rel_str.is_empty() {
                    return root_name;
                } else if !root_name.is_empty() {
                    return format!("{}/{}", root_name, rel_str);
                } else {
                    return rel_str;
                }
            }
        }
        current = dir.parent();
    }

    clean_path.to_string_lossy().replace('\\', "/")
}

/// Renders a Zed-style top breadcrumb bar for an editor pane.
///
/// Layout:
/// - Height: 24px (compact)
/// - Left: Root-relative file path
/// - Right: Outline toggle button (active indicator when outline is visible)
pub fn render_pane_breadcrumb<F>(
    id: impl Into<ElementId>,
    file_path: Option<&Path>,
    is_outline_docked: bool,
    theme: &Theme,
    on_toggle_outline: F,
) -> AnyElement
where
    F: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
{
    let c = &theme.colors;
    let d = &theme.dimensions;
    let path_display = file_path
        .map(format_root_relative_path)
        .unwrap_or_else(|| "Untitled".to_string());

    let id = id.into();
    let btn_id = ElementId::Name(format!("{id:?}-outline-btn").into());

    div()
        .id(id)
        .h(px(24.0))
        .w_full()
        .px(px(8.0))
        .bg(c.editor_background)
        .border_b_1()
        .border_color(c.dialog_border)
        .flex()
        .items_center()
        .justify_between()
        // Left: Path breadcrumb
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .overflow_hidden()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(c.dialog_muted)
                        .whitespace_nowrap()
                        .child(path_display),
                ),
        )
        // Right: Outline toggle button
        .child(
            div()
                .id(btn_id)
                .size(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(d.icon_button_radius))
                .cursor_pointer()
                .when(is_outline_docked, |this| this.bg(c.panel_row_hover))
                .hover(|this| this.bg(c.panel_row_hover))
                .on_mouse_down(MouseButton::Left, on_toggle_outline)
                .child(
                    svg()
                        .path("plugin://splitype.editor/outline/outline.svg")
                        .size(px(13.5))
                        .text_color(if is_outline_docked {
                            c.focus_accent
                        } else {
                            c.dialog_muted
                        }),
                ),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn test_strip_verbatim_prefix() {
        let path = Path::new(r"\\?\C:\Users\test\project\README.md");
        assert_eq!(
            strip_verbatim_prefix(path),
            PathBuf::from(r"C:\Users\test\project\README.md")
        );

        let unc = Path::new(r"\\?\UNC\server\share\file.txt");
        assert_eq!(
            strip_verbatim_prefix(unc),
            PathBuf::from(r"\\server\share\file.txt")
        );

        let normal = Path::new(r"C:\Users\test\project\README.md");
        assert_eq!(strip_verbatim_prefix(normal), PathBuf::from(r"C:\Users\test\project\README.md"));
    }

    #[test]
    fn test_format_root_relative_path_verbatim() {
        let cwd = std::env::current_dir().unwrap();
        let file = cwd.join("README.md");
        let verbatim_file = PathBuf::from(format!(r"\\?\{}", file.display()));
        let formatted = format_root_relative_path(&verbatim_file);
        assert!(!formatted.starts_with(r"\\?\"));
        assert!(formatted.ends_with("README.md"));
    }
}

