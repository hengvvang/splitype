//! Workspace panel state, file-tree scanning, folder management, and outline parsing.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use gpui::*;

use super::Editor;
use crate::blocks::BlockType;
use crate::workspace::Theme;
use crate::workspace::{I18nManager, I18nStrings};

const FOLDER_ICON: &str = "icon/workspace/folder.svg";
const MARKDOWN_ICON: &str = "icon/workspace/markdown.svg";
const FILE_ICON: &str = "icon/workspace/file.svg";
const WORKSPACE_NODE_HEIGHT: f32 = 28.0;
const WORKSPACE_NODE_INDENT: f32 = 14.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceTreeKind {
    Directory(PathBuf),
    MarkdownFile(PathBuf),
    File(PathBuf),
    Heading { line: usize, level: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceTreeNode {
    id: String,
    label: String,
    kind: WorkspaceTreeKind,
    children: Vec<WorkspaceTreeNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceSelection {
    File(PathBuf),
    Outline(String),
}

#[derive(Default)]
pub(crate) struct Explorer {
    pub(crate) is_open: bool,
    pub(crate) root: Option<PathBuf>,
    pub(crate) file_tree: Option<WorkspaceTreeNode>,
    pub(crate) file_error: Option<String>,
    pub(crate) outline_tree: Vec<WorkspaceTreeNode>,
    pub(crate) outline_source: Option<String>,
    pub(crate) expanded: HashSet<String>,
    pub(crate) selected: Option<WorkspaceSelection>,
}

impl Editor {
    pub(crate) fn toggle_workspace_drawer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace.is_open {
            self.workspace.is_open = false;
        } else {
            self.close_menu_bar(cx);
            self.dismiss_contextual_overlays(cx);
            self.workspace.is_open = true;
            self.sync_workspace_models(cx);
            window.activate_window();
        }
        cx.notify();
    }

    pub(crate) fn on_toggle_workspace_action(
        &mut self,
        _: &crate::blocks::ToggleWorkspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_workspace_drawer(window, cx);
    }

    pub(crate) fn sync_workspace_after_document_path_change(&mut self, cx: &mut Context<Self>) {
        if self.workspace.root.is_none() {
            self.workspace.root = self.workspace_root_for_current_file();
        }
        self.workspace.file_tree = None;
        self.workspace.file_error = None;
        self.workspace.outline_source = None;
        if self.workspace.is_open {
            self.sync_workspace_models(cx);
        }
    }

    pub(crate) fn sync_workspace_models(&mut self, cx: &mut Context<Self>) {
        self.sync_workspace_file_tree();
        self.sync_workspace_outline(cx);
    }

    pub(crate) fn workspace_root_for_current_file(&self) -> Option<PathBuf> {
        self.file_path.as_ref()?.parent().map(Path::to_path_buf)
    }

    pub(crate) fn prompt_open_workspace_folder(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        let weak_editor = cx.entity().downgrade();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let paths = match prompt.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(err)) => {
                    eprintln!("[workspace] dialog error: {err}");
                    return;
                }
            };
            let Some(folder_path) = paths.into_iter().next() else {
                return;
            };
            eprintln!("[workspace] selected folder: {folder_path:?}");
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.workspace.root = Some(folder_path);
                editor.workspace.is_open = true;
                editor.workspace.file_tree = None;
                editor.workspace.file_error = None;
                editor.sync_workspace_models(cx);
                if let Some(ref tree) = editor.workspace.file_tree {
                    eprintln!(
                        "[workspace] file_tree: {} children: {}",
                        tree.label,
                        tree.children.len()
                    );
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn create_new_file_in_workspace(
        &mut self,
        parent_dir: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let base_dir = parent_dir
            .or_else(|| self.workspace.root.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let prompt = cx.prompt_for_new_path(&base_dir, Some("untitled.md"));
        let weak_editor = cx.entity().downgrade();
        let window_handle = window.window_handle();

        let _ = cx.spawn(async move |_this, cx| {
            if let Ok(Ok(Some(mut path))) = prompt.await {
                if path.extension().is_none() {
                    path.set_extension("md");
                }
                if let Err(err) = std::fs::write(&path, "") {
                    let detail = err.to_string();
                    let _ = cx.update_window(window_handle, move |_, window, cx| {
                        let strings = cx.global::<I18nManager>().strings().clone();
                        let buttons = [strings.info_dialog_ok.as_str()];
                        let _ = window.prompt(
                            PromptLevel::Critical,
                            "Create File Failed",
                            Some(&detail),
                            &buttons,
                            cx,
                        );
                    });
                    return;
                }
                let path_for_open = path.clone();
                let weak_editor_for_open = weak_editor.clone();
                let _ = weak_editor.update(cx, |editor, cx| {
                    if editor.workspace.root.is_none() {
                        editor.workspace.root = path.parent().map(Path::to_path_buf);
                    }
                    editor.workspace.file_tree = None;
                    editor.sync_workspace_models(cx);
                    let _ = cx.update_window(window_handle, move |_, window, cx| {
                        let _ = weak_editor_for_open.update(cx, |editor, cx| {
                            editor.open_workspace_file(path_for_open, window, cx);
                        });
                    });
                });
            }
        });
    }

    pub(crate) fn create_new_folder_in_workspace(
        &mut self,
        parent_dir: Option<PathBuf>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let base_dir = parent_dir
            .or_else(|| self.workspace.root.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let prompt = cx.prompt_for_new_path(&base_dir, Some("new_folder"));
        let weak_editor = cx.entity().downgrade();

        let _ = cx.spawn(async move |_this, cx| {
            if let Ok(Ok(Some(path))) = prompt.await {
                if let Err(err) = std::fs::create_dir_all(&path) {
                    eprintln!("failed to create directory: {err}");
                    return;
                }
                let _ = weak_editor.update(cx, |editor, cx| {
                    editor.workspace.file_tree = None;
                    editor.sync_workspace_models(cx);
                    cx.notify();
                });
            }
        });
    }

    pub(crate) fn collapse_all_workspace_nodes(&mut self, cx: &mut Context<Self>) {
        self.workspace.expanded.clear();
        if let Some(root) = &self.workspace.file_tree {
            self.workspace.expanded.insert(root.id.clone());
        }
        cx.notify();
    }

    pub(crate) fn refresh_workspace_tree(&mut self, cx: &mut Context<Self>) {
        self.workspace.file_tree = None;
        self.sync_workspace_models(cx);
        cx.notify();
    }

    #[allow(dead_code)]
    pub(crate) fn reveal_in_file_explorer(&self, path: &Path) {
        let path = path.to_path_buf();
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer.exe")
                .arg("/select,")
                .arg(&path)
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(&path)
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let parent = path.parent().unwrap_or(&path);
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn delete_workspace_item(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        let is_dir = path.is_dir();
        let prompt_msg = format!("Are you sure you want to delete '{}'?", item_name);
        let strings = cx.global::<I18nManager>().strings().clone();
        let ok_btn = "Delete";
        let cancel_btn = strings.settings_cancel.clone();

        let prompt = window.prompt(
            PromptLevel::Warning,
            &prompt_msg,
            None,
            &[ok_btn, cancel_btn.as_str()],
            cx,
        );

        let weak_editor = cx.entity().downgrade();
        let _ = cx.spawn(async move |_this, cx| {
            if let Ok(0) = prompt.await {
                let res = if is_dir {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
                if let Err(err) = res {
                    eprintln!("failed to delete item: {err}");
                } else {
                    let _ = weak_editor.update(cx, |editor, cx| {
                        editor.workspace.file_tree = None;
                        editor.sync_workspace_models(cx);
                        cx.notify();
                    });
                }
            }
        });
    }

    pub(crate) fn start_inline_create_file(&mut self, parent: PathBuf, cx: &mut Context<Self>) {
        let default_name = "untitled.md";
        let target_path = parent.join(default_name);
        if std::fs::File::create(&target_path).is_ok() {
            self.refresh_workspace_tree(cx);
            self.workspace.selected = Some(WorkspaceSelection::File(target_path));
        }
    }

    pub(crate) fn start_inline_create_folder(&mut self, parent: PathBuf, cx: &mut Context<Self>) {
        let default_name = "new_folder";
        let target_path = parent.join(default_name);
        if std::fs::create_dir_all(&target_path).is_ok() {
            self.refresh_workspace_tree(cx);
        }
    }

    pub(crate) fn start_inline_rename(&mut self, _target: PathBuf, _cx: &mut Context<Self>) {}

    pub(crate) fn delete_workspace_entry(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
        self.refresh_workspace_tree(cx);
    }

    pub(crate) fn copy_path_to_clipboard(&self, path: &Path, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(
            path.to_string_lossy().to_string(),
        ));
    }

    pub(crate) fn sync_workspace_file_tree(&mut self) {
        if self.workspace.root.is_none() {
            self.workspace.root = self.workspace_root_for_current_file();
        }

        let Some(root) = self.workspace.root.clone() else {
            self.workspace.selected = None;
            return;
        };

        if root.as_os_str().is_empty() {
            self.workspace.file_error = Some("Invalid workspace path: empty path".to_string());
            self.workspace.selected = None;
            return;
        }

        match scan_workspace_dir(&root) {
            Ok(tree) => {
                self.workspace.expanded.insert(tree.id.clone());
                self.workspace.file_tree = Some(tree);
                self.workspace.selected = self
                    .file_path
                    .as_ref()
                    .map(|path| WorkspaceSelection::File(path.clone()));
            }
            Err(err) => {
                self.workspace.file_error = Some(err.to_string());
            }
        }
    }

    pub(crate) fn sync_workspace_outline(&mut self, cx: &mut Context<Self>) {
        let source = self.serialized_document_text(cx);
        if self.workspace.outline_source.as_deref() == Some(source.as_str()) {
            return;
        }

        let outline = build_outline_tree(&source);
        prune_outline_state(&mut self.workspace, &outline);
        self.workspace.outline_tree = outline;
        self.workspace.outline_source = Some(source);
    }

    pub(crate) fn toggle_workspace_node(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.workspace.expanded.remove(id) {
            self.workspace.expanded.insert(id.to_string());
        }
        cx.notify();
    }

    pub(crate) fn select_outline_node(&mut self, id: String, cx: &mut Context<Self>) {
        self.workspace.selected = Some(WorkspaceSelection::Outline(id));
        cx.notify();
    }

    pub(crate) fn open_workspace_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.selected = Some(WorkspaceSelection::File(path.clone()));
        self.request_dropped_markdown_replace(path, window, cx);
    }

    pub(crate) fn render_workspace_files_tree(
        &self,
        theme: &Theme,
        _strings: &I18nStrings,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        if self.workspace.root.is_none() {
            return self.render_workspace_empty_state(
                "Explorer is empty now",
                "Open a folder as the workspace",
                theme,
                editor,
            );
        }

        if let Some(error) = self.workspace.file_error.as_ref() {
            return self.render_workspace_empty_state(
                "Explorer is empty now",
                error,
                theme,
                editor,
            );
        }

        let Some(root) = self.workspace.file_tree.as_ref() else {
            return self.render_workspace_empty_state(
                "Explorer is empty now",
                "Open a folder as the workspace",
                theme,
                editor,
            );
        };

        if root.children.is_empty() {
            return self.render_workspace_empty_state(
                "Explorer is empty now",
                "Open a folder as the workspace",
                theme,
                editor,
            );
        }

        let c = &theme.colors;
        let root_name = self
            .workspace
            .root
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Workspace".to_string());

        let ed_open = editor.clone();
        let ed_file = editor.clone();
        let ed_folder = editor.clone();
        let ed_refresh = editor.clone();
        let ed_collapse = editor.clone();

        let toolbar = div()
            .w_full()
            .h(px(30.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(c.dialog_border)
            .bg(c.dialog_surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        svg()
                            .path("icon/workspace/folder-open.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::BOLD)
                            .text_color(c.text_default)
                            .truncate()
                            .child(root_name),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        div()
                            .id("ws-tb-open")
                            .cursor_pointer()
                            .p(px(3.0))
                            .rounded(px(4.0))
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .child(
                                svg()
                                    .path("icon/workspace/folder-open.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, window, cx| {
                                let _ = ed_open.update(cx, |ed, cx| {
                                    ed.prompt_open_workspace_folder(window, cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("ws-tb-newfile")
                            .cursor_pointer()
                            .p(px(3.0))
                            .rounded(px(4.0))
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .child(
                                svg()
                                    .path("icon/workspace/file-plus.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, window, cx| {
                                let _ = ed_file.update(cx, |ed, cx| {
                                    ed.create_new_file_in_workspace(None, window, cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("ws-tb-newfolder")
                            .cursor_pointer()
                            .p(px(3.0))
                            .rounded(px(4.0))
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .child(
                                svg()
                                    .path("icon/workspace/folder-plus.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, window, cx| {
                                let _ = ed_folder.update(cx, |ed, cx| {
                                    ed.create_new_folder_in_workspace(None, window, cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("ws-tb-refresh")
                            .cursor_pointer()
                            .p(px(3.0))
                            .rounded(px(4.0))
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .child(
                                svg()
                                    .path("icon/workspace/refresh.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, _window, cx| {
                                let _ = ed_refresh.update(cx, |ed, cx| {
                                    ed.refresh_workspace_tree(cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("ws-tb-collapse")
                            .cursor_pointer()
                            .p(px(3.0))
                            .rounded(px(4.0))
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .child(
                                svg()
                                    .path("icon/workspace/collapse-all.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, _window, cx| {
                                let _ = ed_collapse.update(cx, |ed, cx| {
                                    ed.collapse_all_workspace_nodes(cx);
                                });
                            }),
                    ),
            );

        let tree_nodes = div()
            .id("workspace-tree-scroll-container")
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .py(px(4.0))
            .children(self.render_workspace_nodes(&root.children, 0, theme, editor));

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .child(toolbar)
            .child(tree_nodes)
            .into_any_element()
    }

    pub(crate) fn render_workspace_outline_tree(
        &self,
        theme: &Theme,
        strings: &I18nStrings,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        if self.workspace.outline_tree.is_empty() {
            return self.render_workspace_empty_state(
                "",
                &strings.workspace_empty_outline,
                theme,
                editor,
            );
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .children(self.render_workspace_nodes(&self.workspace.outline_tree, 0, theme, editor))
            .into_any_element()
    }

    pub(crate) fn render_workspace_empty_state(
        &self,
        title: &str,
        message: &str,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let click_editor = editor.clone();

        let display_title = if title.is_empty() {
            "Explorer is empty now"
        } else {
            title
        };

        let display_message = if message.is_empty() {
            "Open a folder as the workspace"
        } else {
            message
        };

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .px(px(24.0))
            .text_align(TextAlign::Center)
            .child(
                svg()
                    .path("icon/workspace/folder-open.svg")
                    .size(px(36.0))
                    .text_color(c.dialog_muted),
            )
            .child(
                div()
                    .text_size(px(15.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c.text_default)
                    .child(display_title.to_string()),
            )
            .child(
                div()
                    .max_w(px(230.0))
                    .text_size(px(t.text_size * 0.78))
                    .line_height(px(t.text_size * t.text_line_height * 0.90))
                    .text_color(c.dialog_muted)
                    .child(display_message.to_string()),
            )
            .child(
                div()
                    .id("workspace-empty-open-btn")
                    .cursor_pointer()
                    .mt(px(4.0))
                    .h(px(28.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .rounded(px(d.menu_item_radius))
                    .border_1()
                    .border_color(c.dialog_border)
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .active(|this| this.opacity(0.92))
                    .child(
                        svg()
                            .path("icon/workspace/folder-open.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_secondary_button_text),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c.dialog_secondary_button_text)
                            .child("Open Folder"),
                    )
                    .on_click(move |_ev, window, cx| {
                        let _ = click_editor.update(cx, |ed, cx| {
                            ed.prompt_open_workspace_folder(window, cx);
                        });
                    }),
            )
            .into_any_element()
    }

    pub(crate) fn render_workspace_nodes(
        &self,
        nodes: &[WorkspaceTreeNode],
        depth: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::new();
        for node in nodes {
            elements.push(self.render_workspace_node(node, depth, theme, editor));
            if !node.children.is_empty() && self.workspace.expanded.contains(&node.id) {
                elements.extend(self.render_workspace_nodes(
                    &node.children,
                    depth + 1,
                    theme,
                    editor,
                ));
            }
        }
        elements
    }

    pub(crate) fn render_workspace_node(
        &self,
        node: &WorkspaceTreeNode,
        depth: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let is_expanded = self.workspace.expanded.contains(&node.id);
        let has_children = !node.children.is_empty();
        let selected = match (&self.workspace.selected, &node.kind) {
            (Some(WorkspaceSelection::File(selected)), WorkspaceTreeKind::MarkdownFile(path))
            | (Some(WorkspaceSelection::File(selected)), WorkspaceTreeKind::File(path)) => {
                selected == path
            }
            (Some(WorkspaceSelection::Outline(selected)), _) => selected == &node.id,
            _ => false,
        };
        let node_id = node.id.clone();
        let click_editor = editor.clone();
        let click_kind = node.kind.clone();
        let right_click_editor = editor.clone();
        let right_click_kind = node.kind.clone();
        let arrow_node_id = node.id.clone();
        let arrow_editor = editor.clone();

        let icon = match &node.kind {
            WorkspaceTreeKind::Directory(_) => Some((FOLDER_ICON, Hsla::from(rgba(0xf59e0bff)))),
            WorkspaceTreeKind::MarkdownFile(_) => {
                Some((MARKDOWN_ICON, Hsla::from(rgba(0x2563ebff))))
            }
            WorkspaceTreeKind::File(path) => {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                match ext.as_str() {
                    "rs" | "js" | "ts" | "json" | "toml" | "yaml" | "css" | "html" | "c"
                    | "cpp" | "py" | "go" => Some((FILE_ICON, Hsla::from(rgba(0x10b981ff)))),
                    "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => {
                        Some((FILE_ICON, Hsla::from(rgba(0x8b5cf6ff))))
                    }
                    _ => Some((FILE_ICON, c.dialog_muted)),
                }
            }
            WorkspaceTreeKind::Heading { .. } => None,
        };

        let heading_badge = match &node.kind {
            WorkspaceTreeKind::Heading { level, .. } => {
                let badge_color = match level {
                    1 => c.callout_note_border,
                    2 => c.callout_tip_border,
                    3 => c.callout_important_border,
                    4 => c.callout_warning_border,
                    5 => c.callout_caution_border,
                    _ => c.dialog_muted,
                };
                Some(
                    div()
                        .px(px(4.0))
                        .py(px(1.0))
                        .rounded(px(3.0))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_color)
                        .bg(badge_color.opacity(0.12))
                        .child(format!("H{level}")),
                )
            }
            _ => None,
        };

        let label_color = if selected {
            c.text_default
        } else {
            c.dialog_muted
        };

        let mut arrow_el = div()
            .w(px(14.0))
            .h(px(18.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center();

        if has_children {
            arrow_el = arrow_el
                .cursor_pointer()
                .child(
                    svg()
                        .path(if is_expanded {
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_editor.update(cx, |editor, cx| {
                        editor.toggle_workspace_node(&arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        div()
            .id(("workspace-node", stable_node_hash(&node.id)))
            .h(px(WORKSPACE_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * WORKSPACE_NODE_INDENT))
            .pr(px(8.0))
            .rounded(px(4.0))
            .bg(if selected {
                c.selection
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(arrow_el)
            .children(heading_badge)
            .children(icon.map(|(path, color)| {
                svg()
                    .path(path)
                    .size(px(15.0))
                    .flex_shrink_0()
                    .text_color(color)
                    .into_any_element()
            }))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_size(px(t.text_size * 0.9))
                    .line_height(px(t.text_size * t.text_line_height))
                    .text_color(label_color)
                    .child(node.label.clone()),
            )
            .on_mouse_down(MouseButton::Right, move |event, _window, cx| {
                let node_kind = right_click_kind.clone();
                let _ = right_click_editor.update(cx, |editor, cx| match node_kind {
                    WorkspaceTreeKind::Directory(path) => {
                        editor.open_workspace_file_context_menu(event.position, path, true, cx);
                    }
                    WorkspaceTreeKind::MarkdownFile(path) | WorkspaceTreeKind::File(path) => {
                        editor.open_workspace_file_context_menu(event.position, path, false, cx);
                    }
                    _ => {}
                });
                cx.stop_propagation();
            })
            .on_click(move |_event, window, cx| {
                let node_id = node_id.clone();
                let click_kind = click_kind.clone();
                let _ = click_editor.update(cx, |editor, cx| match click_kind {
                    WorkspaceTreeKind::Directory(_) => {
                        editor.toggle_workspace_node(&node_id, cx);
                    }
                    WorkspaceTreeKind::MarkdownFile(path) | WorkspaceTreeKind::File(path) => {
                        editor.open_workspace_file(path, window, cx);
                    }
                    WorkspaceTreeKind::Heading { .. } => {
                        editor.select_outline_node(node_id, cx);
                    }
                });
            })
            .into_any_element()
    }
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("md"))
}

fn is_ignored_workspace_entry(name: &str) -> bool {
    name == "node_modules"
        || name == "target"
        || name == "dist"
        || name == "build"
        || name == ".git"
}

fn scan_workspace_dir(path: &Path) -> Result<WorkspaceTreeNode> {
    let mut children = Vec::new();
    let read_dir = match fs::read_dir(path) {
        Ok(rd) => rd,
        Err(err) => {
            return Err(anyhow::anyhow!(
                "failed to read '{}': {err}",
                path.display()
            ));
        }
    };

    for entry in read_dir {
        let Ok(entry) = entry else { continue };
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        if is_ignored_workspace_entry(&name) {
            continue;
        }

        let is_dir = entry_path.is_dir();
        let is_file = entry_path.is_file();

        if is_dir {
            let dir_children = scan_workspace_dir(&entry_path)
                .map(|node| node.children)
                .unwrap_or_default();
            children.push(WorkspaceTreeNode {
                id: file_node_id(&entry_path),
                label: name,
                kind: WorkspaceTreeKind::Directory(entry_path),
                children: dir_children,
            });
        } else if is_file {
            let is_md = is_markdown_file(&entry_path);
            let kind = if is_md {
                WorkspaceTreeKind::MarkdownFile(entry_path.clone())
            } else {
                WorkspaceTreeKind::File(entry_path.clone())
            };
            children.push(WorkspaceTreeNode {
                id: file_node_id(&entry_path),
                label: name,
                kind,
                children: Vec::new(),
            });
        }
    }

    children.sort_by(|left, right| {
        let left_dir = matches!(left.kind, WorkspaceTreeKind::Directory(_));
        let right_dir = matches!(right.kind, WorkspaceTreeKind::Directory(_));
        right_dir
            .cmp(&left_dir)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });

    Ok(WorkspaceTreeNode {
        id: file_node_id(path),
        label: file_label(path),
        kind: WorkspaceTreeKind::Directory(path.to_path_buf()),
        children,
    })
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn file_node_id(path: &Path) -> String {
    format!("file:{}", path.to_string_lossy())
}

fn stable_node_hash(id: &str) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

fn prune_outline_state(workspace: &mut Explorer, outline: &[WorkspaceTreeNode]) {
    let mut current_ids = HashSet::new();
    collect_node_ids(outline, &mut current_ids);
    workspace
        .expanded
        .retain(|id| !is_outline_node_id(id) || current_ids.contains(id));

    if matches!(
        &workspace.selected,
        Some(WorkspaceSelection::Outline(id)) if !current_ids.contains(id)
    ) {
        workspace.selected = None;
    }
}

fn collect_node_ids(nodes: &[WorkspaceTreeNode], ids: &mut HashSet<String>) {
    for node in nodes {
        ids.insert(node.id.clone());
        collect_node_ids(&node.children, ids);
    }
}

fn is_outline_node_id(id: &str) -> bool {
    id.starts_with("outline:")
}

fn build_outline_tree(markdown: &str) -> Vec<WorkspaceTreeNode> {
    let mut roots = Vec::new();
    let mut stack: Vec<(u8, Vec<usize>)> = Vec::new();
    let mut fence: Option<(char, usize)> = None;

    for (line_index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some((marker, len)) = fence {
            if is_closing_fence(trimmed, marker, len) {
                fence = None;
            }
            continue;
        }

        if let Some(next_fence) = opening_fence(trimmed) {
            fence = Some(next_fence);
            continue;
        }

        let Some((level, title)) = BlockType::parse_atx_heading_line(line) else {
            continue;
        };

        while stack
            .last()
            .is_some_and(|(parent_level, _)| *parent_level >= level)
        {
            stack.pop();
        }

        let node = WorkspaceTreeNode {
            id: format!("outline:{line_index}"),
            label: title,
            kind: WorkspaceTreeKind::Heading {
                line: line_index,
                level,
            },
            children: Vec::new(),
        };

        let siblings = if let Some((_, parent_path)) = stack.last() {
            children_at_path_mut(&mut roots, parent_path)
        } else {
            &mut roots
        };
        siblings.push(node);

        let mut node_path = stack
            .last()
            .map(|(_, path)| path.clone())
            .unwrap_or_default();
        node_path.push(siblings.len() - 1);
        stack.push((level, node_path));
    }

    roots
}

fn children_at_path_mut<'a>(
    nodes: &'a mut Vec<WorkspaceTreeNode>,
    path: &[usize],
) -> &'a mut Vec<WorkspaceTreeNode> {
    let mut current = nodes;
    for &index in path {
        current = &mut current[index].children;
    }
    current
}

fn opening_fence(trimmed: &str) -> Option<(char, usize)> {
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|ch| *ch == marker).count();
    (len >= 3).then_some((marker, len))
}

fn is_closing_fence(trimmed: &str, marker: char, len: usize) -> bool {
    let count = trimmed.chars().take_while(|ch| *ch == marker).count();
    count >= len && trimmed[count..].trim().is_empty()
}
