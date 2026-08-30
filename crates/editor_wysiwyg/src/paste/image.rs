//! Clipboard image paste helpers: storage resolution, file naming, and markdown insertion formatting.

use std::path::{Path, PathBuf};

use config::settings::ImagePasteBehavior;
use gpui::ImageFormat;

use crate::document::protocol::PastedImageSource;

/// Returns the file extension for a given ImageFormat.
pub fn clipboard_image_extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Webp => "webp",
        ImageFormat::Gif => "gif",
        ImageFormat::Svg => "svg",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Tiff => "tiff",
        ImageFormat::Ico => "ico",
        ImageFormat::Pnm => "pnm",
    }
}

/// Resolves the destination directory for a pasted image given the document path and user policy.
pub fn resolve_image_target_dir(
    behavior: ImagePasteBehavior,
    root_dir: &Path,
    document_path: Option<&Path>,
    source: &PastedImageSource,
) -> anyhow::Result<PathBuf> {
    match behavior {
        ImagePasteBehavior::None | ImagePasteBehavior::CopyToDocumentFolder => {
            Ok(root_dir.to_path_buf())
        }
        ImagePasteBehavior::CopyToAssetsFolder => Ok(root_dir.join("assets")),
        ImagePasteBehavior::CopyToNamedAssetsFolder => {
            let base = document_path
                .and_then(|path| path.file_stem())
                .and_then(|stem| stem.to_str())
                .filter(|stem| !stem.trim().is_empty())
                .unwrap_or("untitle");
            if document_path.is_some() {
                return Ok(root_dir.join(format!("{base}.assets")));
            }

            for index in 0.. {
                let folder = if index == 0 {
                    "untitle.assets".to_string()
                } else {
                    format!("untitle{index}.assets")
                };
                let path = root_dir.join(folder);
                if !path.exists() {
                    return Ok(path);
                }
                if matches!(source, PastedImageSource::LocalPath(_)) {
                    continue;
                }
            }
            unreachable!("unbounded search should always return");
        }
    }
}

/// Computes a non-colliding file path in a directory.
pub fn unique_file_path(dir: &Path, preferred_name: &str) -> PathBuf {
    let preferred = Path::new(preferred_name);
    let stem = preferred
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("image");
    let extension = preferred.extension().and_then(|ext| ext.to_str());
    for index in 0.. {
        let file_name = if index == 0 {
            preferred_name.to_string()
        } else if let Some(extension) = extension {
            format!("{stem}{index}.{extension}")
        } else {
            format!("{stem}{index}")
        };
        let candidate = dir.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded search should always return");
}

/// Formats a Markdown image tag: `![alt](relative_path)`.
pub fn format_image_markdown_tag(alt: &str, relative_path: &str) -> String {
    let normalized = relative_path.replace('\\', "/");
    format!("![{alt}]({normalized})")
}
