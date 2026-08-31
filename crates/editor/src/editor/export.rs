//! Document export flows (HTML, PDF) for the editor.

use std::path::{Path, PathBuf};
use std::thread;

use futures::channel::oneshot;
use gpui::*;

use crate::editor::Editor;
use config::language::I18nManager;
use theme::{Theme, ThemeManager};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Html,
    Pdf,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Pdf => "pdf",
        }
    }
}

#[derive(Debug)]
pub enum ExportError {
    Io(std::io::Error),
    Render(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::Render(msg) => write!(f, "Render error: {msg}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<std::io::Error> for ExportError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl Editor {
    pub fn export_dialog_defaults(&self, format: ExportFormat) -> (PathBuf, String) {
        let extension = format.extension();
        if let Some(path) = self.tab().file.path.as_ref() {
            let directory = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .filter(|stem| !stem.is_empty())
                .unwrap_or("untitled");
            return (directory, format!("{stem}.{extension}"));
        }

        (
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            format!("untitled.{extension}"),
        )
    }

    pub fn export_title(&self) -> String {
        self.tab()
            .file
            .path
            .as_ref()
            .and_then(|path| path.file_stem())
            .map(|stem| stem.to_string_lossy().to_string())
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    pub fn render_export_bytes(
        format: ExportFormat,
        markdown: &str,
        _theme: &Theme,
        title: &str,
        _source_base_dir: Option<&Path>,
    ) -> Result<Vec<u8>, ExportError> {
        match format {
            ExportFormat::Html => {
                let html = format!(
                    r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>{title}</title><style>body {{ font-family: sans-serif; padding: 2rem; max-width: 800px; margin: auto; white-space: pre-wrap; }}</style></head><body>{markdown}</body></html>"#,
                );
                Ok(html.into_bytes())
            }
            ExportFormat::Pdf => {
                Err(ExportError::Render("PDF export is delegated to preview print driver".to_string()))
            }
        }
    }

    pub fn write_export_bytes(
        format: ExportFormat,
        markdown: &str,
        theme: &Theme,
        title: &str,
        path: &Path,
        source_base_dir: Option<&Path>,
    ) -> Result<(), ExportError> {
        let bytes = Self::render_export_bytes(format, markdown, theme, title, source_base_dir)?;
        std::fs::write(path, bytes).map_err(ExportError::Io)
    }

    pub fn export_document_via_prompt(
        &mut self,
        format: ExportFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_tabs() {
            return;
        }
        let markdown = self.serialized_document_text(cx);
        let theme = cx.global::<ThemeManager>().current().clone();
        let title = self.export_title();
        let source_base_dir = self
            .tab()
            .file
            .path
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf);
        let (default_dir, suggested_name) = self.export_dialog_defaults(format);
        let prompt = cx.prompt_for_new_path(&default_dir, Some(&suggested_name));
        let window_handle = window.window_handle();

        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut path = match prompt.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(err)) => {
                    let detail = err.to_string();
                    let _ = cx.update_window(
                        window_handle,
                        move |_view: AnyView, window: &mut Window, cx: &mut App| {
                            show_export_error(window, cx, &detail);
                        },
                    );
                    return;
                }
            };

            if path.extension().is_none() {
                path.set_extension(format.extension());
            }

            let (sender, receiver) = oneshot::channel();
            let spawn_result = thread::Builder::new()
                .name("splitype-export".to_string())
                .spawn(move || {
                    let result = Self::write_export_bytes(
                        format,
                        &markdown,
                        &theme,
                        &title,
                        &path,
                        source_base_dir.as_deref(),
                    );
                    let _ = sender.send(result);
                });

            if let Err(err) = spawn_result {
                let detail = ExportError::Render(err.to_string()).to_string();
                let _ = cx.update_window(
                    window_handle,
                    move |_view: AnyView, window: &mut Window, cx: &mut App| {
                        show_export_error(window, cx, &detail);
                    },
                );
                return;
            }

            let result = receiver
                .await
                .unwrap_or_else(|_| Err(ExportError::Render("export task aborted".to_string())));

            if let Err(err) = result {
                let detail = err.to_string();
                let _ = cx.update_window(
                    window_handle,
                    move |_view: AnyView, window: &mut Window, cx: &mut App| {
                        show_export_error(window, cx, &detail);
                    },
                );
            }
        })
        .detach();
    }
}

fn show_export_error(window: &mut Window, cx: &mut App, detail: &str) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let buttons = [strings.info_dialog_ok.as_str()];
    let _ = window.prompt(
        PromptLevel::Critical,
        &strings.export_failed_title,
        Some(detail),
        &buttons,
        cx,
    );
}

