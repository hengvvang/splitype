#!/usr/bin/env python3
"""Split src/editor/tests.rs into topic modules under src/editor/tests/."""
import re
from pathlib import Path

SRC = Path("src/editor/tests.rs")
DST = Path("src/editor/tests")
lines = SRC.read_text(encoding="utf-8").splitlines(keepends=True)

# 1-based inclusive line ranges for each topic file (content lines only).
RANGES = [
    ("geometry", 109, 268),
    ("about", 270, 296),
    ("save_export", 298, 443),
    ("drop", 446, 640),
    ("window_flows", 642, 1094),
    ("table_ops", 1096, 1499),
    ("image_runtimes", 1501, 2168),
    ("footnotes", 2170, 2373),
    ("view_mode", 2375, 2478),
    ("undo", 2480, 2602),
    ("editing", 2604, 3451),
    ("multi_panel", 3453, 3708),
]

HEADERS = {
    "geometry": "//! Pure layout geometry: centered-column ratio, scrollbar\n//! mapping, and the rendered row window.\n",
    "about": "//! About dialog body and repository link opening.\n",
    "save_export": "//! Save (Ctrl-S / menu action) and HTML export flows.\n",
    "drop": "//! External file drops: clean replace and dirty-drop decisions.\n",
    "window_flows": "//! Window-level flows: menu actions, close guards, quit,\n//! pane-click panel activation.\n",
    "table_ops": "//! Table runtime installation and table manipulation actions.\n",
    "image_runtimes": "//! Image runtime installation and reference resolution across\n//! lists, quotes, callouts, and table cells.\n",
    "footnotes": "//! Footnote numbering and reference binding.\n",
    "view_mode": "//! View-mode toggling: preserved runtimes and positions.\n",
    "undo": "//! Undo / redo history across rendered typing.\n",
    "editing": "//! Keyboard editing: tab/arrow/capture semantics, select-all\n//! cycling, code-block focus, table-cell navigation.\n",
    "multi_panel": "//! Multi-panel isolation: per-panel source runtimes and tab\n//! switching renders the active document.\n",
}

IMPORTS = {
    "geometry": [
        "use crate::editor::controller::Editor;",
        "use crate::infra::theme::Theme;",
        "",
        "use super::*;",
    ],
    "about": [
        "use gpui::TestAppContext;",
        "use crate::infra::i18n::I18nStrings;",
    ],
    "save_export": [
        "use std::fs;",
        "",
        "use crate::editor::actions::SaveDocument;",
        "use crate::editor::controller::Editor;",
        "use crate::editor::render::export::ExportFormat;",
        "use crate::model::inline::text::RichText;",
        "",
        "use super::*;",
    ],
    "drop": [
        "use std::fs;",
        "",
        "use crate::editor::controller::{Editor, EditorMode};",
        "use crate::model::block::BlockKind;",
        "use crate::model::inline::text::RichText;",
        "",
        "use super::*;",
    ],
    "window_flows": [
        "use std::fs;",
        "",
        "use gpui::TestAppContext;",
        "",
        "use crate::app::actions::{CloseWindow, QuitApplication};",
        "use crate::editor::controller::Editor;",
        "",
        "use super::*;",
    ],
    "table_ops": [
        "use gpui::TestAppContext;",
        "",
        "use crate::editor::controller::Editor;",
        "use crate::model::block::BlockKind;",
        "use crate::model::syntax::table::TableColumnAlignment;",
    ],
    "image_runtimes": [
        "use gpui::TestAppContext;",
        "",
        "use crate::editor::controller::Editor;",
        "use crate::model::block::BlockKind;",
        "use crate::model::syntax::image::{",
        "    ImageReferenceDefinitions, ImageResolvedSource, TableCellInlineImageSegment,",
        "    parse_table_cell_inline_images,",
        "};",
        "use std::path::PathBuf;",
    ],
    "footnotes": [
        "use gpui::TestAppContext;",
        "",
        "use crate::editor::controller::Editor;",
        "use crate::model::block::BlockKind;",
        "use crate::model::inline::footnote::superscript_ordinal;",
    ],
    "view_mode": [
        "use gpui::TestAppContext;",
        "",
        "use crate::editor::controller::{Editor, EditorMode};",
    ],
    "undo": ["use gpui::TestAppContext;", "", "use crate::editor::controller::Editor;"],
    "editing": [
        "use gpui::{AppContext, ClickEvent, KeyDownEvent, Keystroke, TestAppContext};",
        "use std::time::{Duration, Instant};",
        "",
        "use crate::editor::controller::{Editor, EditorMode};",
        "use crate::editor::editing::input::actions::{FocusNext, Newline};",
        "use crate::editor::view::context_menu::TableInsertTarget;",
        "use crate::editor::view::dialogs::TableInsertDialogState;",
        "use crate::model::block::BlockKind;",
        "",
        "use super::*;",
    ],
    "multi_panel": [
        "use gpui::TestAppContext;",
        "",
        "use crate::editor::controller::Editor;",
        "use crate::model::block::BlockKind;",
        "",
        "use super::*;",
    ],
}

# The shared helpers live at the top of the old file.
HELPERS_START, HELPERS_END = 24, 107  # 1-based inclusive

def strip_helper_fn(text: str, name: str) -> str:
    """Remove a top-level helper fn from an extracted body."""
    pat = re.compile(rf"^fn {name}\(.*?(?=^(?:#\[|fn |$))", re.S | re.M)
    return pat.sub("", text, count=1)

def extract(start: int, end: int) -> str:
    return "".join(lines[start - 1 : end])

def attr_line(fn_line: int) -> int:
    """Find the attribute line preceding a fn definition (1-based)."""
    i = fn_line - 2  # 0-based index of the line above
    while i >= 0:
        stripped = lines[i].strip()
        if stripped in ("#[test]", "#[gpui::test]"):
            return i + 1
        if stripped.startswith("fn "):
            break
        i -= 1
    return fn_line

# Adjust each range start to include the test attribute line.
RANGES = [(name, attr_line(start), end) for name, start, end in RANGES]

# Build helpers (with uniform_strides moved into the shared module).
helpers = extract(HELPERS_START, HELPERS_END)
helpers = helpers + extract(167, 170)  # uniform_strides

mod_rs = """//! Editor integration-style unit tests, grouped by subsystem.
//!
//! Each topic module below covers one area of the editor; run a single
//! group with `cargo test editor::tests::<module>` (e.g.
//! `cargo test editor::tests::table_ops`). Shared helpers live here.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::TestAppContext;

use crate::editor::controller::Editor;
use crate::infra::i18n::I18nManager;
use crate::infra::theme::ThemeManager;

mod about;
mod drop;
mod editing;
mod footnotes;
mod geometry;
mod image_runtimes;
mod multi_panel;
mod save_export;
mod table_ops;
mod undo;
mod view_mode;
mod window_flows;

""" + helpers

for name, start, end in RANGES:
    body = extract(start, end)
    if name == "geometry":
        body = strip_helper_fn(body, "uniform_strides")
    header = HEADERS[name]
    imports = "\n".join(IMPORTS[name])
    (DST / f"{name}.rs").write_text(
        f"{header}\n{imports}\n\n{body}", encoding="utf-8"
    )
    count = len(re.findall(r"^#\[(?:gpui::)?test\]", body, re.M))
    print(f"{name}.rs: {count} tests, {len(body.splitlines())} lines")

(DST / "mod.rs").write_text(mod_rs, encoding="utf-8")
print("helpers:", len(helpers.splitlines()), "lines")
