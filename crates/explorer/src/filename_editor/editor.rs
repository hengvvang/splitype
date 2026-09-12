//! Native single-line FilenameEditor entity for GPUI.
//!
//! Mirrors Zed's `Entity<Editor>` single-line architecture (`project_panel.rs`):
//! - Persistent GPUI Entity with its own `FocusHandle`
//! - Complete IME composition support through `EntityInputHandler`
//! - Direct keydown handling (Enter/Escape/Backspace/Delete/Arrows/Clipboard)
//! - Event emission (`BufferEdited`, `SelectionsChanged`, `Blurred`, `Confirmed`, `Cancelled`)
//! - Borderless text rendering with horizontal auto-scroll and selection highlight.

use std::cell::{Cell, RefCell};
use std::ops::Range;

use gpui::*;
use theme::ThemeManager;

use crate::state::EXPLORER_NODE_HEIGHT;

// ── UTF-8 / UTF-16 conversions for IME ─────────────────────────────────────

#[inline]
fn utf16_offset_to_utf8(text: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    for (byte_offset, ch) in text.char_indices() {
        if utf16_count >= utf16_offset {
            return byte_offset;
        }
        utf16_count += ch.len_utf16();
    }
    text.len()
}

#[inline]
fn utf8_offset_to_utf16(text: &str, utf8_offset: usize) -> usize {
    let mut utf16_count = 0;
    for (byte_offset, ch) in text.char_indices() {
        if byte_offset >= utf8_offset {
            return utf16_count;
        }
        utf16_count += ch.len_utf16();
    }
    utf16_count
}

#[inline]
fn utf16_range_to_utf8(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = utf16_offset_to_utf8(text, range.start);
    let end = utf16_offset_to_utf8(text, range.end).max(start);
    start..end
}

#[inline]
fn utf8_range_to_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = utf8_offset_to_utf16(text, range.start);
    let end = utf8_offset_to_utf16(text, range.end).max(start);
    start..end
}

// ── Events ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilenameEditorEvent {
    BufferEdited,
    SelectionsChanged,
    Blurred,
    Confirmed,
    Cancelled,
}

// ── FilenameEditor Entity ───────────────────────────────────────────────────

pub struct FilenameEditor {
    pub text: String,
    pub selection: Range<usize>,
    pub reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub focus_handle: Option<FocusHandle>,
    pub last_bounds: Cell<Option<Bounds<Pixels>>>,
    pub last_layout: RefCell<Option<ShapedLine>>,
    pub scroll_offset: Cell<Pixels>,
    pub is_selecting: bool,
    pub opened_at: Option<std::time::Instant>,
}

impl Default for FilenameEditor {
    fn default() -> Self {
        Self {
            text: String::new(),
            selection: 0..0,
            reversed: false,
            marked_range: None,
            focus_handle: None,
            last_bounds: Cell::new(None),
            last_layout: RefCell::new(None),
            scroll_offset: Cell::new(px(0.0)),
            is_selecting: false,
            opened_at: None,
        }
    }
}

impl EventEmitter<FilenameEditorEvent> for FilenameEditor {}

impl FilenameEditor {
    pub fn new(cx: &mut App) -> Self {
        Self {
            text: String::new(),
            selection: 0..0,
            reversed: false,
            marked_range: None,
            focus_handle: Some(cx.focus_handle()),
            last_bounds: Cell::new(None),
            last_layout: RefCell::new(None),
            scroll_offset: Cell::new(px(0.0)),
            is_selecting: false,
            opened_at: None,
        }
    }

    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[inline]
    pub fn selected_text(&self) -> &str {
        let range = self.selection_range();
        &self.text[range]
    }

    #[inline]
    pub fn move_to(&mut self, offset: usize) {
        let offset = offset.min(self.text.len());
        self.selection = offset..offset;
        self.reversed = false;
        self.marked_range = None;
    }

    #[inline]
    pub fn select_to(&mut self, offset: usize) {
        let offset = offset.min(self.text.len());
        let anchor = self.selection_anchor();
        self.set_cursor(offset, anchor, true);
    }

    #[inline]
    pub fn insert_at_selection(&mut self, text: &str) {
        self.insert_text(text);
    }

    #[inline]
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle
            .clone()
            .expect("focus_handle initialized on FilenameEditor")
    }

    #[inline]
    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus_handle
            .as_ref()
            .is_some_and(|h| h.is_focused(window))
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut App) {
        if self.focus_handle.is_none() {
            self.focus_handle = Some(cx.focus_handle());
        }
        let handle = self.focus_handle.clone().unwrap();
        window.focus(&handle, cx);
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.selection = 0..0;
        self.reversed = false;
        self.marked_range = None;
        self.scroll_offset.set(px(0.0));
    }

    pub fn set_text(&mut self, text: String, select: Option<Range<usize>>) {
        self.text = text;
        let len = self.text.len();
        let selection = select.unwrap_or(0..len);
        let start = selection.start.min(len);
        let end = selection.end.min(len).max(start);
        self.selection = start..end;
        self.reversed = false;
        self.marked_range = None;
        self.scroll_offset.set(px(0.0));
    }

    pub fn select_range(&mut self, range: Range<usize>) {
        let len = self.text.len();
        let start = range.start.min(len);
        let end = range.end.min(len).max(start);
        self.selection = start..end;
        self.reversed = false;
        self.marked_range = None;
    }

    #[inline]
    pub fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    #[inline]
    pub fn selection_anchor(&self) -> usize {
        if self.reversed {
            self.selection.end
        } else {
            self.selection.start
        }
    }

    #[inline]
    pub fn selection_range(&self) -> Range<usize> {
        let (start, end) = if self.reversed {
            (self.selection.end, self.selection.start)
        } else {
            (self.selection.start, self.selection.end)
        };
        start..end
    }

    pub fn set_cursor(&mut self, cursor: usize, anchor: usize, extend: bool) {
        let cursor = cursor.min(self.text.len());
        let anchor = anchor.min(self.text.len());
        if extend {
            self.selection = anchor..cursor;
            self.reversed = cursor < anchor;
        } else {
            self.selection = cursor..cursor;
            self.reversed = false;
        }
        self.marked_range = None;
    }

    pub fn select_all(&mut self) {
        self.selection = 0..self.text.len();
        self.reversed = false;
        self.marked_range = None;
    }

    pub fn replace_range(&mut self, range: Range<usize>, new_text: &str) {
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len()).max(start);
        self.text.replace_range(start..end, new_text);
        let cursor = start + new_text.len();
        self.selection = cursor..cursor;
        self.reversed = false;
        self.marked_range = None;
    }

    pub fn insert_text(&mut self, text: &str) {
        let range = self.selection_range();
        self.replace_range(range, text);
    }

    pub fn delete_backward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor > 0 {
            let start = self.text.floor_char_boundary(cursor - 1);
            self.replace_range(start..cursor, "");
        }
    }

    pub fn delete_forward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor < self.text.len() {
            let end = self.text.ceil_char_boundary(cursor + 1).min(self.text.len());
            self.replace_range(cursor..end, "");
        }
    }

    fn is_word_char(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    pub fn prev_word_boundary(&self, offset: usize) -> usize {
        if offset == 0 || self.text.is_empty() {
            return 0;
        }
        let chars: Vec<(usize, char)> = self.text.char_indices().collect();
        let current_idx = chars
            .iter()
            .position(|&(idx, _)| idx >= offset)
            .unwrap_or(chars.len());
        if current_idx == 0 {
            return 0;
        }
        let mut i = current_idx - 1;
        let start_is_word = Self::is_word_char(chars[i].1);
        while i > 0 && Self::is_word_char(chars[i - 1].1) == start_is_word {
            i -= 1;
        }
        chars[i].0
    }

    pub fn next_word_boundary(&self, offset: usize) -> usize {
        if self.text.is_empty() {
            return 0;
        }
        let chars: Vec<(usize, char)> = self.text.char_indices().collect();
        let current_idx = chars
            .iter()
            .position(|&(idx, _)| idx >= offset)
            .unwrap_or(chars.len());
        if current_idx >= chars.len() {
            return self.text.len();
        }
        let mut i = current_idx;
        let start_is_word = Self::is_word_char(chars[i].1);
        while i < chars.len() && Self::is_word_char(chars[i].1) == start_is_word {
            i += 1;
        }
        if i < chars.len() {
            chars[i].0
        } else {
            self.text.len()
        }
    }

    pub fn delete_word_backward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor > 0 {
            let start = self.prev_word_boundary(cursor);
            self.replace_range(start..cursor, "");
        }
    }

    pub fn delete_word_forward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor < self.text.len() {
            let end = self.next_word_boundary(cursor);
            self.replace_range(cursor..end, "");
        }
    }

    pub fn move_left(&mut self, extend: bool) {
        let cursor = self.cursor();
        let target = if cursor > 0 {
            self.text.floor_char_boundary(cursor - 1)
        } else {
            0
        };
        let anchor = self.selection_anchor();
        self.set_cursor(target, anchor, extend);
    }

    pub fn move_right(&mut self, extend: bool) {
        let cursor = self.cursor();
        let target = if cursor < self.text.len() {
            self.text.ceil_char_boundary(cursor + 1).min(self.text.len())
        } else {
            self.text.len()
        };
        let anchor = self.selection_anchor();
        self.set_cursor(target, anchor, extend);
    }

    pub fn move_word_left(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        let target = self.prev_word_boundary(cursor);
        self.set_cursor(target, anchor, extend);
    }

    pub fn move_word_right(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        let target = self.next_word_boundary(cursor);
        self.set_cursor(target, anchor, extend);
    }

    pub fn move_home(&mut self, extend: bool) {
        let anchor = self.selection_anchor();
        self.set_cursor(0, anchor, extend);
    }

    pub fn move_end(&mut self, extend: bool) {
        let anchor = self.selection_anchor();
        self.set_cursor(self.text.len(), anchor, extend);
    }

    pub fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.text.is_empty() {
            return 0;
        }
        let Some(bounds) = self.last_bounds.get() else {
            return 0;
        };
        let layout_ref = self.last_layout.borrow();
        let Some(line) = layout_ref.as_ref() else {
            return 0;
        };
        let rel_x = position.x - bounds.left() + self.scroll_offset.get();
        line.closest_index_for_x(rel_x)
    }

    pub fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        let ctrl = keystroke.modifiers.control || keystroke.modifiers.platform;
        let alt = keystroke.modifiers.alt;
        let shift = keystroke.modifiers.shift;

        // When composing with IME, only Enter, Escape, Backspace break composition
        if self.marked_range.is_some()
            && !matches!(keystroke.key.as_str(), "enter" | "escape" | "backspace")
        {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => {
                cx.stop_propagation();
                cx.emit(FilenameEditorEvent::Confirmed);
                return;
            }
            "escape" => {
                cx.stop_propagation();
                cx.emit(FilenameEditorEvent::Cancelled);
                return;
            }
            "backspace" => {
                cx.stop_propagation();
                if let Some(marked) = self.marked_range.take() {
                    self.replace_range(marked, "");
                } else if ctrl {
                    self.delete_word_backward();
                } else {
                    self.delete_backward();
                }
                cx.emit(FilenameEditorEvent::BufferEdited);
                cx.notify();
                return;
            }
            "delete" => {
                cx.stop_propagation();
                if let Some(marked) = self.marked_range.take() {
                    self.replace_range(marked, "");
                } else if ctrl {
                    self.delete_word_forward();
                } else {
                    self.delete_forward();
                }
                cx.emit(FilenameEditorEvent::BufferEdited);
                cx.notify();
                return;
            }
            "left" => {
                cx.stop_propagation();
                if ctrl {
                    self.move_word_left(shift);
                } else {
                    self.move_left(shift);
                }
                cx.emit(FilenameEditorEvent::SelectionsChanged);
                cx.notify();
                return;
            }
            "right" => {
                cx.stop_propagation();
                if ctrl {
                    self.move_word_right(shift);
                } else {
                    self.move_right(shift);
                }
                cx.emit(FilenameEditorEvent::SelectionsChanged);
                cx.notify();
                return;
            }
            "home" => {
                cx.stop_propagation();
                self.move_home(shift);
                cx.emit(FilenameEditorEvent::SelectionsChanged);
                cx.notify();
                return;
            }
            "end" => {
                cx.stop_propagation();
                self.move_end(shift);
                cx.emit(FilenameEditorEvent::SelectionsChanged);
                cx.notify();
                return;
            }
            _ => {}
        }

        if ctrl && !alt {
            match keystroke.key.to_lowercase().as_str() {
                "a" => {
                    cx.stop_propagation();
                    self.select_all();
                    cx.emit(FilenameEditorEvent::SelectionsChanged);
                    cx.notify();
                }
                "c" => {
                    cx.stop_propagation();
                    let sel = self.selection_range();
                    let text = if !sel.is_empty() {
                        self.text[sel].to_string()
                    } else {
                        self.text.clone()
                    };
                    if !text.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                }
                "x" => {
                    cx.stop_propagation();
                    let range = self.selection_range();
                    if !range.is_empty() {
                        let text = self.text[range.clone()].to_string();
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        self.replace_range(range, "");
                        cx.emit(FilenameEditorEvent::BufferEdited);
                        cx.notify();
                    }
                }
                "v" => {
                    cx.stop_propagation();
                    if let Some(clipboard) = cx.read_from_clipboard()
                        && let Some(text) = clipboard.text()
                    {
                        let sanitized = text.replace(['\r', '\n'], "");
                        let range = self.selection_range();
                        self.replace_range(range, &sanitized);
                        cx.emit(FilenameEditorEvent::BufferEdited);
                        cx.notify();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let idx = self.index_for_mouse_position(event.position);
        match event.click_count {
            1 => {
                self.set_cursor(idx, idx, event.modifiers.shift);
                self.is_selecting = true;
            }
            2 => {
                let start = self.prev_word_boundary(idx);
                let end = self.next_word_boundary(idx);
                self.selection = start..end;
                self.reversed = false;
                self.is_selecting = false;
            }
            _ => {
                self.select_all();
                self.is_selecting = false;
            }
        }
        cx.emit(FilenameEditorEvent::SelectionsChanged);
        cx.notify();
    }

    pub fn handle_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.is_selecting = false;
    }

    pub fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            let idx = self.index_for_mouse_position(event.position);
            let anchor = self.selection_anchor();
            self.set_cursor(idx, anchor, true);
            cx.emit(FilenameEditorEvent::SelectionsChanged);
            cx.notify();
        }
    }
}

// ── EntityInputHandler (IME bridge) ─────────────────────────────────────────

impl EntityInputHandler for FilenameEditor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = utf16_range_to_utf8(&self.text, &range_utf16);
        actual_range.replace(utf8_range_to_utf16(&self.text, &range));
        Some(self.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: utf8_range_to_utf16(&self.text, &self.selection_range()),
            reversed: self.reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| utf8_range_to_utf16(&self.text, range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| utf16_range_to_utf8(&self.text, range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selection_range());
        let sanitized = new_text.replace(['\r', '\n'], "");
        self.replace_range(range, &sanitized);
        cx.emit(FilenameEditorEvent::BufferEdited);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| utf16_range_to_utf8(&self.text, range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selection_range());
        let sanitized = new_text.replace(['\r', '\n'], "");
        let marked = range.start..range.start + sanitized.len();
        let selection = new_selected_range_utf16
            .as_ref()
            .map(|r| utf16_range_to_utf8(&sanitized, r))
            .map(|r| range.start + r.start..range.start + r.end)
            .unwrap_or_else(|| marked.end..marked.end);

        self.replace_range(range, &sanitized);
        self.marked_range = Some(marked);
        self.selection = selection;
        self.reversed = false;
        cx.emit(FilenameEditorEvent::BufferEdited);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout_ref = self.last_layout.borrow();
        let line = layout_ref.as_ref()?;
        let range = utf16_range_to_utf8(&self.text, &range_utf16);
        let start_x = line.x_for_index(range.start) - self.scroll_offset.get();
        let end_x = line.x_for_index(range.end) - self.scroll_offset.get();
        Some(Bounds::from_corners(
            point(bounds.left() + start_x, bounds.top()),
            point(bounds.left() + end_x, bounds.bottom()),
        ))
    }

    fn character_index_for_point(
        &mut self,
        pt: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds.get()?;
        let layout_ref = self.last_layout.borrow();
        let line = layout_ref.as_ref()?;
        let x = pt.x - bounds.left() + self.scroll_offset.get();
        let index = line.closest_index_for_x(x);
        Some(utf8_offset_to_utf16(&self.text, index))
    }
}

// ── Render ──────────────────────────────────────────────────────────────────

impl Render for FilenameEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let is_focused = focus_handle.is_focused(window);

        let entity = cx.entity().clone();
        cx.on_blur(&focus_handle, window, move |_this, window, cx| {
            if window.is_window_active() {
                cx.emit(FilenameEditorEvent::Blurred);
            }
        })
        .detach();

        div()
            .id("filename-editor-host")
            .key_context("FilenameEditor")
            .track_focus(&focus_handle)
            .cursor_text()
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_key_down(event, window, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.handle_mouse_down(event, window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, window, cx| {
                    this.handle_mouse_up(event, window, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                this.handle_mouse_move(event, window, cx);
            }))
            .child(FilenameEditorContentElement {
                editor: entity,
                focus_handle,
                is_focused,
            })
    }
}

// ── FilenameEditorContentElement ────────────────────────────────────────────

struct FilenameEditorContentElement {
    editor: Entity<FilenameEditor>,
    focus_handle: FocusHandle,
    is_focused: bool,
}

struct FilenameEditorPrepaintState {
    line: Option<ShapedLine>,
    selection: Option<PaintQuad>,
    cursor: Option<PaintQuad>,
    hitbox: Option<Hitbox>,
    scroll_offset: Pixels,
}

impl IntoElement for FilenameEditorContentElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for FilenameEditorContentElement {
    type RequestLayoutState = ();
    type PrepaintState = FilenameEditorPrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name("filename-editor-content".into()))
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(EXPLORER_NODE_HEIGHT).into();
        style.flex_grow = 1.0;
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let theme = cx.global::<ThemeManager>().current_arc();
        let editor = self.editor.read(cx);

        editor.last_bounds.set(Some(bounds));

        let text: SharedString = editor.text.clone().into();
        let focused = self.is_focused;

        let base_run = TextRun {
            len: text.len(),
            font: window.text_style().font(),
            color: theme.colors.text_default,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(marked_range) =
            editor.marked_range.as_ref().filter(|_| !text.is_empty())
        {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..base_run.clone()
                },
                TextRun {
                    len: marked_range.end.saturating_sub(marked_range.start),
                    underline: Some(UnderlineStyle {
                        color: Some(theme.colors.text_default),
                        thickness: px(theme.dimensions.underline_thickness),
                        wavy: false,
                    }),
                    ..base_run.clone()
                },
                TextRun {
                    len: text.len().saturating_sub(marked_range.end),
                    ..base_run
                },
            ]
            .into_iter()
            .filter(|r| r.len > 0)
            .collect()
        } else {
            vec![base_run]
        };

        let font_size = px(theme.typography.text_size * 0.9);
        let line = window
            .text_system()
            .shape_line(text, font_size, &runs, None);
        let line_height = bounds.size.height;
        let selection_range = editor.selection_range();

        let raw_cursor_x = line.x_for_index(if editor.reversed {
            editor.selection.start
        } else {
            editor.selection.end
        });

        // Compute horizontal scroll offset so cursor stays in view
        let available_w = bounds.size.width.max(px(0.0));
        let mut scroll_offset = editor.scroll_offset.get();
        if raw_cursor_x - scroll_offset > available_w {
            scroll_offset = raw_cursor_x - available_w;
        } else if raw_cursor_x - scroll_offset < px(0.0) {
            scroll_offset = raw_cursor_x;
        }
        let total_w = line.width();
        if total_w <= available_w {
            scroll_offset = px(0.0);
        } else if scroll_offset > total_w - available_w {
            scroll_offset = total_w - available_w;
        }
        let scroll_offset = scroll_offset.max(px(0.0));
        editor.scroll_offset.set(scroll_offset);

        let selection = if focused && !selection_range.is_empty() {
            let start = line.x_for_index(selection_range.start) - scroll_offset;
            let end = line.x_for_index(selection_range.end) - scroll_offset;
            let sel_height = font_size * 1.35;
            let sel_y = bounds.top() + (line_height - sel_height) / 2.0;
            Some(fill(
                Bounds::from_corners(
                    point(bounds.left() + start, sel_y),
                    point(bounds.left() + end, sel_y + sel_height),
                ),
                theme.colors.selection,
            ))
        } else {
            None
        };

        let cursor = if focused && selection_range.is_empty() {
            let cursor_x = raw_cursor_x - scroll_offset;
            let mut cursor_color = theme.colors.cursor;
            cursor_color.a = 1.0;
            let cursor_height = font_size * 1.25;
            let cursor_y = bounds.top() + (line_height - cursor_height) / 2.0;
            Some(fill(
                Bounds::new(
                    point(bounds.left() + cursor_x, cursor_y),
                    size(px(theme.dimensions.cursor_width.max(1.5)), cursor_height),
                ),
                cursor_color,
            ))
        } else {
            None
        };

        let hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        FilenameEditorPrepaintState {
            line: Some(line),
            selection,
            cursor,
            hitbox,
            scroll_offset,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(hitbox) = prepaint.hitbox.as_ref()
            && hitbox.is_hovered(window)
        {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
        }

        if self.is_focused {
            window.handle_input(
                &self.focus_handle,
                ElementInputHandler::new(bounds, self.editor.clone()),
                cx,
            );
        }

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if let Some(selection) = prepaint.selection.take() {
                window.paint_quad(selection);
            }

            if let Some(line) = prepaint.line.take() {
                *self.editor.read(cx).last_layout.borrow_mut() = Some(line.clone());
                let scroll_offset = prepaint.scroll_offset;
                let origin = point(bounds.origin.x - scroll_offset, bounds.origin.y);
                line.paint(
                    origin,
                    bounds.size.height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
            }

            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        });
    }
}
