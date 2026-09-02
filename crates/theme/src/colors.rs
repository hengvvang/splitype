//! Color palette and semantic color tokens.

use gpui::Hsla;
use serde::{Deserialize, Serialize};

theme_section!(
    colors
    /// All configurable colors for the editor UI.
    struct ThemeColors,
    /// Partial color overrides; every `None` field inherits from the base
    /// theme during resolution.
    struct ThemeColorsPatch,
    {
        /// Background of the editor scroll area (behind all blocks).
        editor_background: Hsla,
        /// Background of the focused raw block in source-editing mode.
        source_mode_block_bg: Hsla,
        /// Background used for visible Markdown comment blocks.
        comment_bg: Hsla,
        /// Default paragraph / body text colour.
        text_default: Hsla,
        /// Inline link text colour in rendered mode.
        text_link: Hsla,
        /// Colour of projected Markdown delimiter markers (`**`, `~`, `^`, `[^`,
        /// backticks) revealed while editing the source of a focused block.
        markdown_marker: Hsla,
        /// Placeholder text shown in empty focused blocks.
        text_placeholder: Hsla,
        /// H1 heading text colour.
        text_h1: Hsla,
        /// H2 heading text colour.
        text_h2: Hsla,
        /// H3 heading text colour.
        text_h3: Hsla,
        /// H4 heading text colour.
        text_h4: Hsla,
        /// H5 heading text colour.
        text_h5: Hsla,
        /// H6 heading text colour.
        text_h6: Hsla,
        /// H1 bottom-border colour.
        border_h1: Hsla,
        /// H2 bottom-border colour.
        border_h2: Hsla,
        /// Quote block text colour.
        text_quote: Hsla,
        /// Quote block left-border colour.
        border_quote: Hsla,
        /// Note callout background.
        callout_note_bg: Hsla,
        /// Note callout accent border/text colour.
        callout_note_border: Hsla,
        /// Tip callout background.
        callout_tip_bg: Hsla,
        /// Tip callout accent border/text colour.
        callout_tip_border: Hsla,
        /// Important callout background.
        callout_important_bg: Hsla,
        /// Important callout accent border/text colour.
        callout_important_border: Hsla,
        /// Warning callout background.
        callout_warning_bg: Hsla,
        /// Warning callout accent border/text colour.
        callout_warning_border: Hsla,
        /// Caution callout background.
        callout_caution_bg: Hsla,
        /// Caution callout accent border/text colour.
        callout_caution_border: Hsla,
        /// Border colour of the collected footnotes section divider in preview.
        footnote_border: Hsla,
        /// Back-reference colour inside footnote rows.
        footnote_backref: Hsla,
        /// Border colour of interactive task-list checkboxes.
        task_checkbox_border: Hsla,
        /// Background of unchecked task-list checkboxes.
        task_checkbox_bg: Hsla,
        /// Background of checked task-list checkboxes.
        task_checkbox_checked_bg: Hsla,
        /// Checkmark colour inside checked task-list checkboxes.
        task_checkbox_check: Hsla,
        /// Colour of the separator block line.
        separator: Hsla,
        /// Background of inline code and code-block quads.
        code_bg: Hsla,
        /// Background highlight colour for ==highlight== text.
        text_highlight_bg: Hsla,
        /// Text colour inside code blocks.
        code_text: Hsla,
        /// Background of the focused code-block language input.
        code_language_input_bg: Hsla,
        /// Border colour of the focused code-block language input.
        code_language_input_border: Hsla,
        /// Text colour of the focused code-block language input.
        code_language_input_text: Hsla,
        /// Placeholder colour of the focused code-block language input.
        code_language_input_placeholder: Hsla,
        /// Syntax colour for comments inside code blocks.
        code_syntax_comment: Hsla,
        /// Syntax colour for keywords inside code blocks.
        code_syntax_keyword: Hsla,
        /// Syntax colour for strings inside code blocks.
        code_syntax_string: Hsla,
        /// Syntax colour for numbers inside code blocks.
        code_syntax_number: Hsla,
        /// Syntax colour for types and modules inside code blocks.
        code_syntax_type: Hsla,
        /// Syntax colour for functions and constructors inside code blocks.
        code_syntax_function: Hsla,
        /// Syntax colour for constants inside code blocks.
        code_syntax_constant: Hsla,
        /// Syntax colour for variables and parameters inside code blocks.
        code_syntax_variable: Hsla,
        /// Syntax colour for properties and attributes inside code blocks.
        code_syntax_property: Hsla,
        /// Syntax colour for operators inside code blocks.
        code_syntax_operator: Hsla,
        /// Syntax colour for punctuation inside code blocks.
        code_syntax_punctuation: Hsla,
        /// Border colour of native table cells.
        table_border: Hsla,
        /// Background of native table header cells.
        table_header_bg: Hsla,
        /// Background of native table body cells.
        table_cell_bg: Hsla,
        /// Outline colour of the active native table cell.
        table_cell_active_outline: Hsla,
        /// Preview highlight colour for row/column table-axis selection bands.
        table_axis_preview_bg: Hsla,
        /// Selected highlight colour for row/column table-axis selection bands.
        table_axis_selected_bg: Hsla,
        /// Background of rendered-mode native table append controls.
        table_append_button_bg: Hsla,
        /// Hover background of rendered-mode native table append controls.
        table_append_button_hover: Hsla,
        /// Text colour of rendered-mode native table append controls.
        table_append_button_text: Hsla,
        /// Background of table handle pills (row/column drag handles).
        table_handle_bg: Hsla,
        /// Icon colour of table handle dots.
        table_handle_icon: Hsla,
        /// Border colour of the selection frame around selected table columns/rows.
        table_selection_border: Hsla,
        /// Background of image placeholders in rendered mode.
        image_placeholder_bg: Hsla,
        /// Border colour of image placeholders in rendered mode.
        image_placeholder_border: Hsla,
        /// Text colour of image placeholders in rendered mode.
        image_placeholder_text: Hsla,
        /// Caption text colour shown below rendered images.
        image_caption_text: Hsla,
        /// Scrollbar thumb colour (auto-fading overlay).
        scrollbar_thumb: Hsla,
        /// Text-editing cursor (caret) colour.
        cursor: Hsla,
        /// Text-selection highlight colour.
        selection: Hsla,
        /// Active drag and focus accent colour (drag lines, active pills, indicators).
        focus_accent: Hsla,
        /// Split-preview indicator colour (drag-to-split lines and highlights).
        split_indicator: Hsla,
        /// Semi-transparent backdrop behind the unsaved-changes dialog.
        dialog_backdrop: Hsla,
        /// Background of the unsaved-changes dialog.
        dialog_surface: Hsla,
        /// Border colour of the unsaved-changes dialog.
        dialog_border: Hsla,
        /// Title text colour in the unsaved-changes dialog.
        dialog_title: Hsla,
        /// Body text colour in the unsaved-changes dialog.
        dialog_body: Hsla,
        /// Muted / hint text colour in the unsaved-changes dialog.
        dialog_muted: Hsla,
        /// Primary (save-and-close) button background.
        dialog_primary_button_bg: Hsla,
        /// Primary button hover background.
        dialog_primary_button_hover: Hsla,
        /// Primary button text colour.
        dialog_primary_button_text: Hsla,
        /// Secondary (cancel) button background.
        dialog_secondary_button_bg: Hsla,
        /// Secondary button text colour.
        dialog_secondary_button_text: Hsla,
        /// App-menu button icon colour while the menu bar is expanded.
        app_menu_active: Hsla,
        /// Danger (discard-and-close) button background.
        dialog_danger_button_bg: Hsla,
        /// Danger button hover background.
        dialog_danger_button_hover: Hsla,
        /// Danger button text colour.
        dialog_danger_button_text: Hsla,
        /// Background of the editor status bar.
        bottombar_background: Hsla,
        /// Primary text colour in the status bar.
        bottombar_text: Hsla,
        /// Dimmed/secondary text colour in the status bar.
        bottombar_text_dim: Hsla,
        /// Subtle row hover background (file tree, outline, menus, dropdowns, tabs).
        /// Also the selected-row background: selection is indicated by the accent
        /// indicator bar, so selected rows share the hover highlight.
        panel_row_hover: Hsla,
    }
);
