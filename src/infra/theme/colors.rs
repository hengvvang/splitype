//! Color palette and semantic color tokens.

use gpui::Hsla;
use serde::{Deserialize, Serialize};

/// All configurable colors for the editor UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    /// Background of the editor scroll area (behind all blocks).
    pub editor_background: Hsla,
    /// Background of the focused raw block in source-editing mode.
    pub source_mode_block_bg: Hsla,
    /// Background used for visible Markdown comment blocks.
    pub comment_bg: Hsla,
    /// Default paragraph / body text colour.
    pub text_default: Hsla,
    /// Inline link text colour in rendered mode.
    pub text_link: Hsla,
    /// Colour of projected Markdown delimiter markers (`**`, `~`, `^`, `[^`,
    /// backticks) revealed while editing the source of a focused block.
    pub markdown_marker: Hsla,
    /// Placeholder text shown in empty focused blocks.
    pub text_placeholder: Hsla,
    /// H1 heading text colour.
    pub text_h1: Hsla,
    /// H2 heading text colour.
    pub text_h2: Hsla,
    /// H3 heading text colour.
    pub text_h3: Hsla,
    /// H4 heading text colour.
    pub text_h4: Hsla,
    /// H5 heading text colour.
    pub text_h5: Hsla,
    /// H6 heading text colour.
    pub text_h6: Hsla,
    /// H1 bottom-border colour.
    pub border_h1: Hsla,
    /// H2 bottom-border colour.
    pub border_h2: Hsla,
    /// Quote block text colour.
    pub text_quote: Hsla,
    /// Quote block left-border colour.
    pub border_quote: Hsla,
    /// Note callout background.
    pub callout_note_bg: Hsla,
    /// Note callout accent border/text colour.
    pub callout_note_border: Hsla,
    /// Tip callout background.
    pub callout_tip_bg: Hsla,
    /// Tip callout accent border/text colour.
    pub callout_tip_border: Hsla,
    /// Important callout background.
    pub callout_important_bg: Hsla,
    /// Important callout accent border/text colour.
    pub callout_important_border: Hsla,
    /// Warning callout background.
    pub callout_warning_bg: Hsla,
    /// Warning callout accent border/text colour.
    pub callout_warning_border: Hsla,
    /// Caution callout background.
    pub callout_caution_bg: Hsla,
    /// Caution callout accent border/text colour.
    pub callout_caution_border: Hsla,
    /// Border colour of the collected footnotes section divider in preview.
    pub footnote_border: Hsla,
    /// Back-reference colour inside footnote rows.
    pub footnote_backref: Hsla,
    /// Border colour of interactive task-list checkboxes.
    pub task_checkbox_border: Hsla,
    /// Background of unchecked task-list checkboxes.
    pub task_checkbox_bg: Hsla,
    /// Background of checked task-list checkboxes.
    pub task_checkbox_checked_bg: Hsla,
    /// Checkmark colour inside checked task-list checkboxes.
    pub task_checkbox_check: Hsla,
    /// Colour of the separator block line.
    pub separator: Hsla,
    /// Background of inline code and code-block quads.
    pub code_bg: Hsla,
    /// Text colour inside code blocks.
    pub code_text: Hsla,
    /// Background of the focused code-block language input.
    pub code_language_input_bg: Hsla,
    /// Border colour of the focused code-block language input.
    pub code_language_input_border: Hsla,
    /// Text colour of the focused code-block language input.
    pub code_language_input_text: Hsla,
    /// Placeholder colour of the focused code-block language input.
    pub code_language_input_placeholder: Hsla,
    /// Syntax colour for comments inside code blocks.
    pub code_syntax_comment: Hsla,
    /// Syntax colour for keywords inside code blocks.
    pub code_syntax_keyword: Hsla,
    /// Syntax colour for strings inside code blocks.
    pub code_syntax_string: Hsla,
    /// Syntax colour for numbers inside code blocks.
    pub code_syntax_number: Hsla,
    /// Syntax colour for types and modules inside code blocks.
    pub code_syntax_type: Hsla,
    /// Syntax colour for functions and constructors inside code blocks.
    pub code_syntax_function: Hsla,
    /// Syntax colour for constants inside code blocks.
    pub code_syntax_constant: Hsla,
    /// Syntax colour for variables and parameters inside code blocks.
    pub code_syntax_variable: Hsla,
    /// Syntax colour for properties and attributes inside code blocks.
    pub code_syntax_property: Hsla,
    /// Syntax colour for operators inside code blocks.
    pub code_syntax_operator: Hsla,
    /// Syntax colour for punctuation inside code blocks.
    pub code_syntax_punctuation: Hsla,
    /// Border colour of native table cells.
    pub table_border: Hsla,
    /// Background of native table header cells.
    pub table_header_bg: Hsla,
    /// Background of native table body cells.
    pub table_cell_bg: Hsla,
    /// Outline colour of the active native table cell.
    pub table_cell_active_outline: Hsla,
    /// Preview highlight colour for row/column table-axis selection bands.
    pub table_axis_preview_bg: Hsla,
    /// Selected highlight colour for row/column table-axis selection bands.
    pub table_axis_selected_bg: Hsla,
    /// Background of rendered-mode native table append controls.
    pub table_append_button_bg: Hsla,
    /// Hover background of rendered-mode native table append controls.
    pub table_append_button_hover: Hsla,
    /// Text colour of rendered-mode native table append controls.
    pub table_append_button_text: Hsla,
    /// Background of table handle pills (row/column drag handles).
    pub table_handle_bg: Hsla,
    /// Icon colour of table handle dots.
    pub table_handle_icon: Hsla,
    /// Border colour of the selection frame around selected table columns/rows.
    pub table_selection_border: Hsla,
    /// Background of image placeholders in rendered mode.
    pub image_placeholder_bg: Hsla,
    /// Border colour of image placeholders in rendered mode.
    pub image_placeholder_border: Hsla,
    /// Text colour of image placeholders in rendered mode.
    pub image_placeholder_text: Hsla,
    /// Caption text colour shown below rendered images.
    pub image_caption_text: Hsla,
    /// Scrollbar thumb colour (auto-fading overlay).
    pub scrollbar_thumb: Hsla,
    /// Text-editing cursor (caret) colour.
    pub cursor: Hsla,
    /// Text-selection highlight colour.
    pub selection: Hsla,
    /// Active drag accent — splitter-bar drag highlight (#72cffe sky blue).
    pub focus_accent: Hsla,
    /// Split-preview indicator colour (drag-to-split lines and highlights).
    pub split_indicator: Hsla,
    /// Semi-transparent backdrop behind the unsaved-changes dialog.
    pub dialog_backdrop: Hsla,
    /// Background of the unsaved-changes dialog.
    pub dialog_surface: Hsla,
    /// Border colour of the unsaved-changes dialog.
    pub dialog_border: Hsla,
    /// Title text colour in the unsaved-changes dialog.
    pub dialog_title: Hsla,
    /// Body text colour in the unsaved-changes dialog.
    pub dialog_body: Hsla,
    /// Muted / hint text colour in the unsaved-changes dialog.
    pub dialog_muted: Hsla,
    /// Primary (save-and-close) button background.
    pub dialog_primary_button_bg: Hsla,
    /// Primary button hover background.
    pub dialog_primary_button_hover: Hsla,
    /// Primary button text colour.
    pub dialog_primary_button_text: Hsla,
    /// Secondary (cancel) button background.
    pub dialog_secondary_button_bg: Hsla,
    /// Secondary button hover background.
    pub dialog_secondary_button_hover: Hsla,
    /// Secondary button text colour.
    pub dialog_secondary_button_text: Hsla,
    /// App-menu button icon colour while the menu bar is expanded.
    pub app_menu_active: Hsla,
    /// Danger (discard-and-close) button background.
    pub dialog_danger_button_bg: Hsla,
    /// Danger button hover background.
    pub dialog_danger_button_hover: Hsla,
    /// Danger button text colour.
    pub dialog_danger_button_text: Hsla,
    /// Background of the editor status bar.
    pub bottombar_background: Hsla,
    /// Primary text colour in the status bar.
    pub bottombar_text: Hsla,
    /// Dimmed/secondary text colour in the status bar.
    pub bottombar_text_dim: Hsla,
    /// Hover background for clickable status bar items.
    pub bottombar_button_hover: Hsla,
    /// Explorer panel row hover background (translucent highlight).
    pub panel_row_hover: Hsla,
    /// Explorer panel row selection background (light blue).
    pub panel_row_selected: Hsla,
}

