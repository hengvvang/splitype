//! editor_model — the editor family's contract and vocabulary layer.
//!
//! This crate owns the *contract* types every editor mode (WYSIWYG,
//! Source Code, Preview) and every consumer (outline, search, export)
//! depends on — and nothing else: the pane-kind vocabulary, pane ids,
//! the session primitives (tab kinds, open modes), the outline node
//! type, the `Pane` plugin trait, the `PaneHost` reverse seam, and the
//! [`EditorHost`] dependency-inversion seam to the window shell.
//!
//! Dependency direction: modes and consumers depend on `editor_model`;
//! `editor_model` depends on neither. The `Editor` entity lives in the
//! app composition root (ADR-01) and talks to the modes only through
//! these contracts and the reverse seams.

pub use gpui;

mod autoscroll;
mod pane_factory;
mod pane_host;

pub use autoscroll::AutoscrollStrategy;
pub use pane_factory::{PaneFactory, PaneFactoryRegistry};
pub use pane_host::{PaneHost, PaneRenderContext};

/// The pane kinds an Editor panel can host: the document views
/// inside its split tree. The tree holds only real views — the welcome
/// state is the area's mode, not a panel kind — so the split structure
/// survives tab open/close cycles unchanged and the remembered panel
/// layout needs no migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorPaneKind {
    /// Raw Markdown source code editor.
    SourceCode,
    /// Visual block editor (WYSIWYG rendered view).
    Wysiwyg,
    /// Read-only rendered Markdown preview.
    Preview,
}

impl EditorPaneKind {
    #[inline]
    pub fn is_wysiwyg(&self) -> bool {
        matches!(self, Self::Wysiwyg)
    }

    #[inline]
    pub fn is_source_code(&self) -> bool {
        matches!(self, Self::SourceCode)
    }

    #[inline]
    pub fn is_preview(&self) -> bool {
        matches!(self, Self::Preview)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::SourceCode => "Source Code",
            Self::Wysiwyg => "Wysiwyg",
            Self::Preview => "Preview",
        }
    }

    /// All editor pane types (status-bar dropdown options).
    pub fn all() -> &'static [EditorPaneKind] {
        &[Self::Wysiwyg, Self::Preview, Self::SourceCode]
    }
}

/// The strongly-typed identifier of one inner tiled editor pane
/// (WYSIWYG, Source Code, Preview).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PaneId(pub splitter::tree::NodeId);

impl From<splitter::tree::NodeId> for PaneId {
    #[inline]
    fn from(id: splitter::tree::NodeId) -> Self {
        Self(id)
    }
}

impl From<PaneId> for splitter::tree::NodeId {
    #[inline]
    fn from(id: PaneId) -> Self {
        id.0
    }
}

impl From<PaneId> for gpui::ElementId {
    #[inline]
    fn from(id: PaneId) -> Self {
        id.0.into()
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle retention kind of a document tab in an editor pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TabKind {
    /// Transient temporary tab: replaced in-place when another file is clicked.
    #[default]
    Transient,
    /// Persistent resident tab: pinned to the tab bar until explicitly closed.
    Persistent,
}

/// Requested mode when opening a file into an editor pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OpenFileMode {
    /// Open as transient tab (replaces existing clean transient tab if present).
    #[default]
    Transient,
    /// Open as persistent tab (or promotes existing tab to persistent).
    Persistent,
}

/// A heading node in the outline HUD (pure data; owned by `editor` so both
/// the outline panel and the modes can name it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineNode {
    pub id: String,
    pub label: String,
    pub level: u8,
    pub block_index: usize,
    pub block_id: Option<gpui::EntityId>,
}

use std::path::Path;

use gpui::{App, Window};

/// Service contract between the editor family and the window shell
/// (dependency inversion seam).
///
/// The editor family never names the shell type: the `Editor` entity
/// holds an `Arc<dyn EditorHost>` and the app's composition root injects
/// a `ShellEditorHost` (defined next to `Shell`) when it spawns editor
/// entities. Every shell-side capability the editor needs goes through
/// this trait, so the editor crates depend on nothing above them in the
/// dependency graph and can be exercised with a no-op host in tests.
/// Window-scoped work that must run after an editor update finishes is
/// deferred by the editor; the host itself never re-enters the editor.
///
/// All methods take `&mut App` (never a shell context) so implementations
/// can be invoked from deferred app callbacks without naming the shell
/// type. Methods that need a `Window` receive it as an argument.
pub trait EditorHost: Send + Sync + 'static {
    /// Bring the window panel `panel_id` to the foreground.
    fn activate_panel(&self, panel_id: workspace::PanelId, cx: &mut App);

    /// Toggle the window panel's kind dropdown (top bar control).
    fn toggle_panel_dropdown(&self, panel_id: workspace::PanelId, cx: &mut App);

    /// Split the window panel into two editor panels along `axis`.
    fn split_panel(
        &self,
        panel_id: workspace::PanelId,
        axis: splitter::SplitAxis,
        ratio: f32,
        copy_content: bool,
        cx: &mut App,
    );

    /// Maximize or restore the window panel.
    fn toggle_panel_maximize(&self, panel_id: workspace::PanelId, cx: &mut App);

    /// Request closing the window panel (runs the shell's dirty check).
    fn request_close_panel(&self, panel_id: workspace::PanelId, cx: &mut App);

    /// Prompt the shell's unsaved-changes dialog for one tab.
    fn prompt_close_tab(&self, panel_id: workspace::PanelId, index: usize, cx: &mut App);

    /// Open `path` in the active editor tab of the shell.
    fn open_file_in_active_editor(
        &self,
        path: &Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut App,
    ) -> bool;

    /// Dismiss the shell's info dialog (drop-replace flow).
    fn hide_info_dialog(&self, cx: &mut App);

    /// Close window-level layout dropdowns opened by the shell.
    fn clear_outer_dropdowns(&self, cx: &mut App);

    /// Keep the explorer selection in sync after a document path change
    /// (the explorer is a sibling panel; the editor must not name it).
    fn sync_explorer_after_document_path_change(&self, cx: &mut App);

    /// Record a recently opened document path (the app's recent-files
    /// menu is a window-shell concern; the editor must not name it).
    fn record_recent_file(&self, path: &Path, cx: &mut App);
}

// ── Outline heading extraction (Pane contract service) ──────────────────

/// Parse an ATX heading line (`### Title`) into (level, content).
///
/// Lightweight extraction for outline consumers; the full markdown
/// parser in the WYSIWYG world has its own richer block grammar. This is
/// deliberately a small, self-contained recognizer so every mode can
/// answer "what are my headings" without depending on a parser crate.
pub fn parse_atx_heading_line(line: &str) -> Option<(u8, String)> {
    let trimmed_end = line.trim_end();
    let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
    if leading_spaces > 3 {
        return None;
    }
    let rest = &trimmed_end[leading_spaces..];
    let level = rest.bytes().take_while(|b| *b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let after_hashes = &rest[level..];
    let content = if after_hashes.is_empty() {
        ""
    } else if let Some(stripped) = after_hashes.strip_prefix(' ') {
        stripped
    } else if let Some(stripped) = after_hashes.strip_prefix('\t') {
        stripped
    } else {
        return None;
    };
    let mut content = content.trim_end().to_string();
    if let Some(closing_hash_start) = content.rfind(' ')
        && content[closing_hash_start + 1..]
            .chars()
            .all(|ch| ch == '#')
    {
        content.truncate(closing_hash_start);
        content = content.trim_end().to_string();
    } else if !content.is_empty() && content.chars().all(|ch| ch == '#') {
        content.clear();
    }
    Some((level as u8, content))
}

/// Parse a Setext underline (`=` or `-` sequence) into the heading level.
pub fn parse_setext_underline(line: &str) -> Option<u8> {
    let trimmed_end = line.trim_end();
    let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
    if leading_spaces > 3 {
        return None;
    }
    let rest = &trimmed_end[leading_spaces..];
    if rest.is_empty() {
        return None;
    }
    if rest.bytes().all(|b| b == b'=') {
        Some(1)
    } else if rest.bytes().all(|b| b == b'-') {
        Some(2)
    } else {
        None
    }
}

/// Extract all heading items from raw Markdown text (ATX + Setext
/// headings; fenced code blocks are skipped so `#` inside ``` isn't
/// treated as a heading).
pub fn outline_headings_from_markdown(markdown: &str) -> Vec<OutlineNode> {
    let mut list = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_fence = false;
    let mut fence_char = '`';
    let mut fence_len = 3;

    let mut line_idx = 0;
    while line_idx < lines.len() {
        let line = lines[line_idx];
        let trimmed = line.trim_start();

        if in_fence {
            if trimmed.starts_with(fence_char) {
                let count = trimmed.chars().take_while(|&c| c == fence_char).count();
                if count >= fence_len && trimmed[count..].trim().is_empty() {
                    in_fence = false;
                }
            }
            line_idx += 1;
            continue;
        } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence_char = trimmed.chars().next().unwrap_or('`');
            fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
            in_fence = true;
            line_idx += 1;
            continue;
        }

        // ATX heading: `# Heading`
        if let Some((level, raw_text)) = parse_atx_heading_line(line) {
            let label = raw_text.trim().to_string();
            list.push(OutlineNode {
                id: format!("outline:line:{line_idx}"),
                label: if label.is_empty() {
                    format!("Heading {level}")
                } else {
                    label
                },
                level,
                block_index: line_idx,
                block_id: None,
            });
            line_idx += 1;
            continue;
        }

        // Setext heading: `Heading Line\n===` or `Heading Line\n---`
        if line_idx + 1 < lines.len() && !trimmed.is_empty() {
            let next_line = lines[line_idx + 1].trim_start();
            if let Some(level) = parse_setext_underline(next_line) {
                let label = trimmed.to_string();
                list.push(OutlineNode {
                    id: format!("outline:line:{line_idx}"),
                    label: if label.is_empty() {
                        format!("Heading {level}")
                    } else {
                        label
                    },
                    level,
                    block_index: line_idx,
                    block_id: None,
                });
                line_idx += 2;
                continue;
            }
        }

        line_idx += 1;
    }
    list
}

/// Minimal document view the editor modes may read.
///
/// Implemented by the `Editor` entity (and by test doubles). The modes
/// use it for cross-mode data (serialized markdown, outline headings)
/// without naming the entity type, keeping the Pane contract free of a
/// concrete editor.
pub trait EditorDocument {
    /// Serialize the active document to markdown.
    fn serialize_markdown(&self, cx: &App) -> String;

    /// Outline headings parsed from the document's block structure.
    fn outline_headings(&self, cx: &App) -> Vec<OutlineNode>;
}

use std::any::Any;
use std::ops::Range;

/// The plugin contract implemented by every editor pane kind.
///
/// Every view mode (WYSIWYG, Source Code, Preview) implements [`Pane`];
/// the editor holds one pane state per split leaf (`Box<dyn Pane>`) and
/// talks to the modes only through this trait. Cross-mode consumers
/// (export, search, outline) read *pure data* through
/// [`Pane::document_source`] / [`Pane::outline_items`] and push *pure
/// ranges* through [`Pane::set_search_matches`] — no mode internals ever
/// cross a crate boundary.
pub trait Pane: Any {
    /// Which pane kind this state belongs to.
    fn kind(&self) -> EditorPaneKind;

    /// Pure markdown source of the active tab, as this mode sees it.
    ///
    /// Export, search and outline consume this; the mode decides what
    /// "source" means (WYSIWYG serializes the block tree, Source Code
    /// returns its raw buffer, Preview serializes the shared document).
    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String;

    /// Push in-pane search matches as pure byte ranges (range, is-active).
    ///
    /// Modes highlight these in their own rendering. WYSIWYG highlights at
    /// the block level (the editor syncs `block.search_matches`) and
    /// Preview is read-only, so both are no-ops by design.
    fn set_search_matches(&mut self, matches: &[(Range<usize>, bool)]);

    /// Heading items for the outline HUD (pure data).
    fn outline_items(&self, doc: &dyn EditorDocument, cx: &App) -> Vec<OutlineNode>;

    /// Type-erased access for downcasting to the concrete mode state.
    fn as_any(&self) -> &dyn Any;

    /// Type-erased mutable access for downcasting to the concrete mode state.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[cfg(test)]
mod editor_tests;
