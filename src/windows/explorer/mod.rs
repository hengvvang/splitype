//! Explorer — file-tree sidebar with create/rename/delete actions.

pub(crate) mod state;

use crate::ui::components::button::icon_chip_button;

use std::path::{Path, PathBuf};

use gpui::*;

use crate::editor::actions::{CloseExplorerFolder, ToggleExplorer};
use crate::editor::controller::Editor;
use crate::infra::config::recent::{read_recent_files, read_recent_folders};
use crate::infra::i18n::{I18nManager, I18nStrings};
use crate::theme::Theme;
use crate::ui::components::empty_state::empty_state_container;
use crate::windows::explorer::state::*;

impl Editor {
    pub(crate) fn toggle_explorer_drawer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.panels.explorer.is_open {
            self.panels.explorer.is_open = false;
        } else {
            self.close_menu_bar(cx);
            self.dismiss_contextual_overlays(cx);
            self.panels.explorer.is_open = true;
            self.sync_explorer_models(cx);
            window.activate_window();
        }
        cx.notify();
    }
    pub(crate) fn on_toggle_explorer_action(
        &mut self,
        _: &ToggleExplorer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_explorer_drawer(window, cx);
    }
    pub(crate) fn on_close_explorer_folder_action(
        &mut self,
        _: &CloseExplorerFolder,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_explorer_folder(cx);
    }
    pub(crate) fn close_explorer_folder(&mut self, cx: &mut Context<Self>) {
        self.panels.explorer.root = None;
        self.panels.explorer.file_tree = None;
        self.panels.explorer.file_error = None;
        self.panels.explorer.outline_tree = Vec::new();
        self.panels.explorer.outline_source = None;
        self.panels.explorer.expanded.clear();
        self.panels.explorer.selected = None;
        cx.notify();
    }
    pub(crate) fn open_explorer_folder_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.panels.explorer.root = Some(path);
        self.panels.explorer.is_open = true;
        self.panels.explorer.file_tree = None;
        self.panels.explorer.file_error = None;
        self.sync_explorer_models(cx);
        cx.notify();
    }
    pub(crate) fn sync_explorer_after_document_path_change(&mut self, cx: &mut Context<Self>) {
        if self.panels.explorer.root.is_none() {
            self.panels.explorer.root = self.explorer_root_for_current_file();
        }
        self.panels.explorer.file_tree = None;
        self.panels.explorer.file_error = None;
        self.panels.explorer.outline_source = None;
        if self.panels.explorer.is_open {
            self.sync_explorer_models(cx);
        }
    }
    pub(crate) fn sync_explorer_models(&mut self, cx: &mut Context<Self>) {
        // The file tree only needs a root directory, so it syncs even in
        // the welcome state (no tabs). The outline reads the active
        // document and only runs once a tab exists.
        self.sync_explorer_file_tree();
        if self.has_active_tab() {
            self.sync_explorer_outline(cx);
        }
    }
    pub(crate) fn explorer_root_for_current_file(&self) -> Option<PathBuf> {
        self.active_editor_tab()
            .and_then(|tab| tab.file.path.as_ref())
            .and_then(|path| path.parent().map(Path::to_path_buf))
    }
    pub(crate) fn prompt_open_explorer_folder(
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
                    eprintln!("[explorer] dialog error: {err}");
                    return;
                }
            };
            let Some(folder_path) = paths.into_iter().next() else {
                return;
            };
            eprintln!("[explorer] selected folder: {folder_path:?}");
            if let Err(err) = crate::infra::config::recent::record_recent_folder(&folder_path) {
                eprintln!("failed to update recent folder history: {err}");
            }
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.open_explorer_folder_path(folder_path, cx);
                cx.notify();
            });
        })
        .detach();
    }
    pub(crate) fn create_new_file_in_explorer(
        &mut self,
        parent_dir: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let base_dir = parent_dir
            .or_else(|| self.panels.explorer.root.clone())
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
                    if editor.panels.explorer.root.is_none() {
                        editor.panels.explorer.root = path.parent().map(Path::to_path_buf);
                    }
                    editor.panels.explorer.file_tree = None;
                    editor.sync_explorer_models(cx);
                    let _ = cx.update_window(window_handle, move |_, window, cx| {
                        let _ = weak_editor_for_open.update(cx, |editor, cx| {
                            editor.open_explorer_file(path_for_open, window, cx);
                        });
                    });
                });
            }
        });
    }
    pub(crate) fn create_new_folder_in_explorer(
        &mut self,
        parent_dir: Option<PathBuf>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let base_dir = parent_dir
            .or_else(|| self.panels.explorer.root.clone())
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
                    editor.panels.explorer.file_tree = None;
                    editor.sync_explorer_models(cx);
                    cx.notify();
                });
            }
        });
    }
    pub(crate) fn collapse_all_explorer_nodes(&mut self, cx: &mut Context<Self>) {
        self.panels.explorer.expanded.clear();
        if let Some(root) = &self.panels.explorer.file_tree {
            self.panels.explorer.expanded.insert(root.id.clone());
        }
        cx.notify();
    }
    pub(crate) fn refresh_explorer_tree(&mut self, cx: &mut Context<Self>) {
        self.panels.explorer.file_tree = None;
        self.sync_explorer_models(cx);
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
    pub(crate) fn delete_explorer_item(
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
                        editor.panels.explorer.file_tree = None;
                        editor.sync_explorer_models(cx);
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
            self.refresh_explorer_tree(cx);
            self.panels.explorer.selected = Some(ExplorerSelection::File(target_path));
        }
    }
    pub(crate) fn start_inline_create_folder(&mut self, parent: PathBuf, cx: &mut Context<Self>) {
        let default_name = "new_folder";
        let target_path = parent.join(default_name);
        if std::fs::create_dir_all(&target_path).is_ok() {
            self.refresh_explorer_tree(cx);
        }
    }
    pub(crate) fn start_inline_rename(&mut self, _target: PathBuf, _cx: &mut Context<Self>) {}
    pub(crate) fn delete_explorer_entry(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
        self.refresh_explorer_tree(cx);
    }
    pub(crate) fn copy_path_to_clipboard(&self, path: &Path, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(
            path.to_string_lossy().to_string(),
        ));
    }
    pub(crate) fn sync_explorer_file_tree(&mut self) {
        if self.panels.explorer.root.is_none() {
            self.panels.explorer.root = self.explorer_root_for_current_file();
        }

        let Some(root) = self.panels.explorer.root.clone() else {
            self.panels.explorer.selected = None;
            return;
        };

        if root.as_os_str().is_empty() {
            self.panels.explorer.file_error = Some("Invalid explorer path: empty path".to_string());
            self.panels.explorer.selected = None;
            return;
        }

        match scan_explorer_dir(&root) {
            Ok(tree) => {
                self.panels.explorer.expanded.insert(tree.id.clone());
                self.panels.explorer.file_tree = Some(tree);
                self.panels.explorer.selected = self
                    .active_editor_tab()
                    .and_then(|tab| tab.file.path.as_ref())
                    .map(|path| ExplorerSelection::File(path.clone()));
            }
            Err(err) => {
                self.panels.explorer.file_error = Some(err.to_string());
            }
        }
    }
    pub(crate) fn toggle_explorer_node(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.panels.explorer.expanded.remove(id) {
            self.panels.explorer.expanded.insert(id.to_string());
        }
        cx.notify();
    }
    pub(crate) fn open_explorer_file(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.explorer.selected = Some(ExplorerSelection::File(path.clone()));
        // Explorer interacts with the ACTIVE editor: the file opens in its
        // tab bar. With no Editor area present the click is ignored.
        if self.panels.layout.active_editor_area.is_none() {
            return;
        }
        self.open_file_in_active_editor(&path, window, cx);
    }
    pub(crate) fn render_explorer_files_tree(
        &self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        // Recent files and folders give the empty state a quick-open entry
        // point; stale history entries are filtered out so clicks never fail.
        let recent_folders = read_recent_folders()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_dir())
            .take(5)
            .collect::<Vec<_>>();
        let recent_files = read_recent_files()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_file())
            .take(5)
            .collect::<Vec<_>>();

        if self.panels.explorer.root.is_none() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                editor,
            );
        }

        if let Some(error) = self.panels.explorer.file_error.as_ref() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                error,
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                editor,
            );
        }

        let Some(root) = self.panels.explorer.file_tree.as_ref() else {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                editor,
            );
        };

        if root.children.is_empty() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                editor,
            );
        }

        let c = &theme.colors;
        let d = &theme.dimensions;
        let root_name = self
            .panels
            .explorer
            .root
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Explorer".to_string());

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
                            .path("icon/explorer/folder-open.svg")
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
                        icon_chip_button(c, d)
                            .id(("ws-tb-open", area_id))
                            .child(
                                svg()
                                    .path("icon/explorer/folder-open.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, window, cx| {
                                let _ = ed_open.update(cx, |ed, cx| {
                                    ed.prompt_open_explorer_folder(window, cx);
                                });
                            }),
                    )
                    .child(
                        icon_chip_button(c, d)
                            .id(("ws-tb-newfile", area_id))
                            .child(
                                svg()
                                    .path("icon/explorer/file-plus.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, window, cx| {
                                let _ = ed_file.update(cx, |ed, cx| {
                                    ed.create_new_file_in_explorer(None, window, cx);
                                });
                            }),
                    )
                    .child(
                        icon_chip_button(c, d)
                            .id(("ws-tb-newfolder", area_id))
                            .child(
                                svg()
                                    .path("icon/explorer/folder-plus.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, window, cx| {
                                let _ = ed_folder.update(cx, |ed, cx| {
                                    ed.create_new_folder_in_explorer(None, window, cx);
                                });
                            }),
                    )
                    .child(
                        icon_chip_button(c, d)
                            .id(("ws-tb-refresh", area_id))
                            .child(
                                svg()
                                    .path("icon/explorer/refresh.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, _window, cx| {
                                let _ = ed_refresh.update(cx, |ed, cx| {
                                    ed.refresh_explorer_tree(cx);
                                });
                            }),
                    )
                    .child(
                        icon_chip_button(c, d)
                            .id(("ws-tb-collapse", area_id))
                            .child(
                                svg()
                                    .path("icon/explorer/collapse-all.svg")
                                    .size(px(14.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_click(move |_ev, _window, cx| {
                                let _ = ed_collapse.update(cx, |ed, cx| {
                                    ed.collapse_all_explorer_nodes(cx);
                                });
                            }),
                    ),
            );

        let tree_nodes = div()
            .id(("explorer-tree-scroll-container", area_id))
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .py(px(4.0))
            .children(self.render_explorer_nodes(&root.children, 0, area_id, theme, editor));

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .child(toolbar)
            .child(tree_nodes)
            .into_any_element()
    }
    pub(crate) fn render_explorer_empty_state(
        &self,
        title: &str,
        message: &str,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        recent_folders: &[PathBuf],
        recent_files: &[PathBuf],
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

        // An empty message means the empty state has no hint line at all;
        // non-empty messages (e.g. scan errors) are still rendered.
        let has_message = !message.is_empty();

        empty_state_container()
            .gap(px(10.0))
            .px(px(24.0))
            .child(
                svg()
                    .path("icon/explorer/folder-open.svg")
                    .size(px(36.0))
                    .text_color(c.dialog_muted),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c.text_default)
                    .child(display_title.to_string()),
            )
            .child(if has_message {
                div()
                    .max_w(px(230.0))
                    .text_size(px(t.text_size * 0.78))
                    .line_height(px(t.text_size * t.text_line_height * 0.90))
                    .text_color(c.dialog_muted)
                    .child(message.to_string())
            } else {
                div()
            })
            .child(
                div()
                    .id(("explorer-empty-open-btn", area_id))
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
                            .path("icon/explorer/folder-open.svg")
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
                            ed.prompt_open_explorer_folder(window, cx);
                        });
                    }),
            )
            .child(
                // Recent folders and files quick-open list under the button;
                // hidden when both histories are empty or the state carries
                // an error message.
                if (recent_folders.is_empty() && recent_files.is_empty()) || has_message {
                    div()
                } else {
                    div()
                        .mt(px(16.0))
                        .w_full()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(2.0))
                        .child(
                            div()
                                .ml(px(10.0))
                                .text_size(px(13.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(c.dialog_muted)
                                .child(strings.explorer_recent_title.clone()),
                        )
                        .children(recent_folders.iter().map(|path| {
                            let folder_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());
                            let ed = editor.clone();
                            let path = path.clone();
                            div()
                                .id(ElementId::Name(
                                    format!(
                                        "explorer-recent-folder-{}-{}",
                                        area_id,
                                        path.display()
                                    )
                                    .into(),
                                ))
                                .cursor_pointer()
                                .px(px(10.0))
                                .py(px(2.0))
                                .rounded(px(d.menu_item_radius))
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    svg()
                                        .path("icon/explorer/folder.svg")
                                        .size(px(12.0))
                                        .text_color(c.dialog_muted),
                                )
                                .child(
                                    div()
                                        .max_w(px(190.0))
                                        .truncate()
                                        .text_size(px(12.0))
                                        .text_color(c.dialog_muted)
                                        .hover(|this| this.text_color(c.text_default))
                                        .child(folder_name),
                                )
                                .on_click(move |_, _window, cx| {
                                    let _ = ed.update(cx, |editor, cx| {
                                        editor.open_explorer_folder_path(path.clone(), cx);
                                    });
                                })
                        }))
                        .children(recent_files.iter().map(|path| {
                            let file_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());
                            let ed = editor.clone();
                            let path = path.clone();
                            div()
                                .id(ElementId::Name(
                                    format!("explorer-recent-{}-{}", area_id, path.display()).into(),
                                ))
                                .cursor_pointer()
                                .px(px(10.0))
                                .py(px(2.0))
                                .rounded(px(d.menu_item_radius))
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    svg()
                                        .path("icon/explorer/markdown.svg")
                                        .size(px(12.0))
                                        .text_color(c.dialog_muted),
                                )
                                .child(
                                    div()
                                        .max_w(px(190.0))
                                        .truncate()
                                        .text_size(px(12.0))
                                        .text_color(c.dialog_muted)
                                        .hover(|this| this.text_color(c.text_default))
                                        .child(file_name),
                                )
                                .on_click(move |_, window, cx| {
                                    let _ = ed.update(cx, |editor, cx| {
                                        editor.open_explorer_file(path.clone(), window, cx);
                                    });
                                })
                        }))
                },
            )
            .into_any_element()
    }
    pub(crate) fn render_explorer_nodes(
        &self,
        nodes: &[ExplorerNode],
        depth: usize,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::new();
        for node in nodes {
            elements.push(self.render_explorer_node(node, depth, area_id, theme, editor));
            if !node.children.is_empty() && self.panels.explorer.expanded.contains(&node.id) {
                elements.extend(self.render_explorer_nodes(
                    &node.children,
                    depth + 1,
                    area_id,
                    theme,
                    editor,
                ));
            }
        }
        elements
    }
    pub(crate) fn render_explorer_node(
        &self,
        node: &ExplorerNode,
        depth: usize,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let is_expanded = self.panels.explorer.expanded.contains(&node.id);
        let has_children = !node.children.is_empty();
        let selected = match (&self.panels.explorer.selected, &node.kind) {
            (Some(ExplorerSelection::File(selected)), ExplorerNodeKind::MarkdownFile(path))
            | (Some(ExplorerSelection::File(selected)), ExplorerNodeKind::File(path)) => {
                selected == path
            }
            (Some(ExplorerSelection::Outline(selected)), _) => selected == &node.id,
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
            ExplorerNodeKind::Directory(_) => Some((FOLDER_ICON, Hsla::from(rgba(0xf59e0bff)))),
            ExplorerNodeKind::MarkdownFile(_) => {
                Some((MARKDOWN_ICON, Hsla::from(rgba(0x2563ebff))))
            }
            ExplorerNodeKind::File(path) => {
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
            ExplorerNodeKind::Heading { .. } => None,
        };

        let heading_badge = match &node.kind {
            ExplorerNodeKind::Heading { level, .. } => {
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
                        editor.toggle_explorer_node(&arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        div()
            .id(ElementId::Name(
                format!("explorer-node-{area_id}-{}", stable_node_hash(&node.id)).into(),
            ))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
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
                    ExplorerNodeKind::Directory(path) => {
                        editor.open_explorer_file_context_menu(event.position, path, true, cx);
                    }
                    ExplorerNodeKind::MarkdownFile(path) | ExplorerNodeKind::File(path) => {
                        editor.open_explorer_file_context_menu(event.position, path, false, cx);
                    }
                    _ => {}
                });
                cx.stop_propagation();
            })
            .on_click(move |_event, window, cx| {
                let node_id = node_id.clone();
                let click_kind = click_kind.clone();
                let _ = click_editor.update(cx, |editor, cx| match click_kind {
                    ExplorerNodeKind::Directory(_) => {
                        editor.toggle_explorer_node(&node_id, cx);
                    }
                    ExplorerNodeKind::MarkdownFile(path) | ExplorerNodeKind::File(path) => {
                        editor.open_explorer_file(path, window, cx);
                    }
                    ExplorerNodeKind::Heading { .. } => {
                        editor.select_outline_node(node_id, cx);
                    }
                });
            })
            .into_any_element()
    }
    pub(crate) fn render_explorer_panel(
        &mut self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_explorer_models(cx);
        let editor = cx.entity().downgrade();
        self.render_explorer_files_tree(area_id, theme, strings, &editor)
    }
}
