//! Application asset loader for bundled SVG icons.

use std::borrow::Cow;

use gpui::*;

pub(crate) struct VelotypeAssets;

impl AssetSource for VelotypeAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        match path {
            "icon/workspace/folder.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/workspace/folder.svg"
            )))),
            "icon/workspace/markdown.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/workspace/markdown.svg"
            )))),
            "icon/workspace/file.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/workspace/file.svg"
            )))),
            "icon/workspace/folder-open.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/workspace/folder-open.svg"
            )))),
            "icon/workspace/file-plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/workspace/file-plus.svg"
            )))),
            "icon/workspace/folder-plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/workspace/folder-plus.svg"
            )))),
            "icon/workspace/collapse-all.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/workspace/collapse-all.svg"
            )))),
            "icon/workspace/refresh.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/workspace/refresh.svg"
            )))),
            "icon/titlebar/chrome-close.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/titlebar/chrome-close.svg"
            )))),
            "icon/titlebar/chrome-minimize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/titlebar/chrome-minimize.svg"
            )))),
            "icon/titlebar/chrome-maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/titlebar/chrome-maximize.svg"
            )))),
            "icon/titlebar/chrome-restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/titlebar/chrome-restore.svg"
            )))),
            "icon/panel/split-h.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/split-h.svg"
            )))),
            "icon/panel/split-v.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/split-v.svg"
            )))),
            "icon/panel/chevron-right.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/chevron-right.svg"
            )))),
            "icon/panel/chevron-down.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/chevron-down.svg"
            )))),
            "icon/panel/select-chevron.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/select-chevron.svg"
            )))),
            "icon/panel/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/check.svg"
            )))),
            "icon/task_check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/task_check.svg"
            )))),
            "icon/panel/sun.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/sun.svg"
            )))),
            "icon/panel/moon.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/moon.svg"
            )))),
            "icon/panel/line-numbers.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/panel/line-numbers.svg"
            )))),
            "icon/table/plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/table/plus.svg"
            )))),
            "icon/table/handle-row.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/table/handle-row.svg"
            )))),
            "icon/table/handle-row-hollow.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/table/handle-row-hollow.svg"
            )))),
            "icon/table/handle-row-solid.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/table/handle-row-solid.svg"
            )))),
            "icon/table/handle-column.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/table/handle-column.svg"
            )))),
            "icon/callout/note.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/callout/note.svg"
            )))),
            "icon/callout/tip.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/callout/tip.svg"
            )))),
            "icon/callout/important.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/callout/important.svg"
            )))),
            "icon/callout/warning.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/callout/warning.svg"
            )))),
            "icon/callout/caution.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/icon/callout/caution.svg"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
