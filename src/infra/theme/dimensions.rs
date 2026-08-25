//! Spacing, sizing, and layout dimensions.

use serde::{Deserialize, Serialize};

/// Global geometric corner radius primitives (WinUI 3 / Fluent Design System)
pub const OVERLAY_CORNER_RADIUS: f32 = 8.0;
pub const CONTROL_CORNER_RADIUS: f32 = 4.0;
pub const SMALL_CONTROL_CORNER_RADIUS: f32 = 2.0;
pub const FULL_CORNER_RADIUS: f32 = 999.0;
pub const SEAMLESS_CORNER_RADIUS: f32 = 0.0;

/// All configurable dimensions (paddings, gaps, sizes) for the editor UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDimensions {
    /// Padding around the editor content area.
    pub editor_padding: f32,
    /// Vertical gap between adjacent blocks.
    pub block_gap: f32,
    /// Minimum height of every block.
    pub block_min_height: f32,
    /// Vertical padding inside each block.
    pub block_padding_y: f32,
    /// Horizontal padding inside each block.
    pub block_padding_x: f32,
    /// Extra horizontal indent per nesting level (list items).
    pub nested_block_indent: f32,
    /// Gap between list marker and its text content.
    pub list_marker_gap: f32,
    /// Minimum width of the bullet list marker column.
    pub list_marker_width: f32,
    /// Minimum width of the ordered-list marker column.
    pub ordered_list_marker_width: f32,
    /// Width and height of the interactive task-list checkbox.
    pub task_checkbox_size: f32,
    /// Corner radius of the task-list checkbox.
    pub task_checkbox_radius: f32,
    /// Border width of the task-list checkbox.
    pub task_checkbox_border_width: f32,
    /// Checkmark font size inside the task-list checkbox.
    pub task_checkbox_check_size: f32,
    /// Extra padding below H1 text.
    pub h1_padding_bottom: f32,
    /// Margin below the H1 bottom border.
    pub h1_margin_bottom: f32,
    /// Width of the text-editing cursor (caret).
    pub cursor_width: f32,
    /// Thickness of the underline decoration.
    pub underline_thickness: f32,
    /// H1 bottom-border thickness.
    pub h1_border_width: f32,
    /// Quote block left-border thickness.
    pub quote_border_width: f32,
    /// Extra left padding between quote border and text.
    pub quote_padding_left: f32,
    /// Horizontal padding inside editor-level callout shells.
    pub callout_padding_x: f32,
    /// Vertical padding inside editor-level callout shells.
    pub callout_padding_y: f32,
    /// Vertical gap between callout body rows.
    pub callout_body_gap: f32,
    /// Corner radius of editor-level callout shells.
    pub callout_radius: f32,
    /// Accent border width of editor-level callout shells.
    pub callout_border_width: f32,
    /// Gap between callout icon and header text.
    pub callout_header_gap: f32,
    /// Vertical margin between the callout header row and the first body row.
    pub callout_header_margin_bottom: f32,
    /// Thickness of the separator block line.
    pub separator_thickness: f32,
    /// Extra horizontal inset applied to separator blocks.
    pub separator_inset_x: f32,
    /// Vertical margin around separator blocks.
    pub separator_margin_y: f32,
    /// Vertical padding inside a code block.
    pub code_block_padding_y: f32,
    /// Horizontal padding inside a code block.
    pub code_block_padding_x: f32,
    /// Corner radius of fenced/raw code block containers.
    pub code_block_radius: f32,
    /// Horizontal padding around inline code background quads.
    pub code_bg_pad_x: f32,
    /// Vertical padding around inline code background quads.
    pub code_bg_pad_y: f32,
    /// Corner radius for inline code background quads.
    pub code_bg_radius: f32,
    /// Width of the code-block language input.
    pub code_language_input_width: f32,
    /// Text layout height inside the code-block language input.
    pub code_language_input_height: f32,
    /// Horizontal padding inside the code-block language input.
    pub code_language_input_padding_x: f32,
    /// Vertical padding inside the code-block language input.
    pub code_language_input_padding_y: f32,
    /// Corner radius of the code-block language input.
    pub code_language_input_radius: f32,
    /// Border width of the code-block language input.
    pub code_language_input_border_width: f32,
    /// Gap between code text and the language input.
    pub code_language_input_gap: f32,
    /// Horizontal padding inside native table cells.
    pub table_cell_padding_x: f32,
    /// Vertical padding inside native table cells.
    pub table_cell_padding_y: f32,
    /// Minimum height of native table cells.
    pub table_cell_min_height: f32,
    /// Width of the append-column control and height of the append-row control.
    pub table_append_button_extent: f32,
    /// Inset padding around rendered-mode native table append controls.
    pub table_append_button_inset: f32,
    /// Invisible activation overlap that keeps append controls easy to hover.
    pub table_append_activation_band: f32,
    /// Outer corner radius of the table container.
    pub table_border_radius: f32,
    /// Corner radius of table drag handles.
    pub table_handle_radius: f32,
    /// Width of the table handle pill.
    pub table_handle_width: f32,
    /// Height of the table handle pill.
    pub table_handle_height: f32,
    /// Border thickness of the selection frame around selected columns/rows.
    pub table_selection_border_width: f32,
    /// Corner radius of rendered images and image placeholders.
    pub image_radius: f32,
    /// Maximum height of rendered root-paragraph images.
    pub image_root_max_height: f32,
    /// Maximum height of rendered table-cell images.
    pub image_cell_max_height: f32,
    /// Default placeholder height for rendered root-paragraph images.
    pub image_root_placeholder_height: f32,
    /// Default placeholder height for rendered table-cell images.
    pub image_cell_placeholder_height: f32,
    /// Vertical gap between a rendered image and its caption.
    pub image_caption_gap: f32,
    /// Width of the custom scrollbar thumb.
    pub scrollbar_width: f32,
    /// Distance of the scrollbar thumb from the right edge.
    pub scrollbar_right: f32,
    /// Viewport width at which the content column starts shrinking.
    pub centered_shrink_start: f32,
    /// Viewport width at which the content column reaches minimum ratio.
    pub centered_shrink_end: f32,
    /// Minimum content-column width as a fraction of available width.
    pub centered_min_ratio: f32,
    /// Width of the unsaved-changes dialog.
    pub dialog_width: f32,
    /// Padding inside the unsaved-changes dialog.
    pub dialog_padding: f32,
    /// Gap between dialog sections.
    pub dialog_gap: f32,
    /// Corner radius of modal dialog cards.
    pub dialog_radius: f32,
    /// Border width of the unsaved-changes dialog.
    pub dialog_border_width: f32,
    /// Height of dialog action buttons.
    pub dialog_button_height: f32,
    /// Gap between dialog action buttons.
    pub dialog_button_gap: f32,
    /// Horizontal padding inside dialog action buttons.
    pub dialog_button_padding_x: f32,
    /// Corner radius of standard action buttons.
    pub button_radius: f32,
    /// Corner radius of toolbar and header icon buttons.
    pub icon_button_radius: f32,
    /// Corner radius of editor and settings navigation tabs.
    pub tab_radius: f32,
    /// Corner radius of the 12px micro tab close button.
    pub tab_close_button_radius: f32,
    /// Height reserved for the in-window fallback menu bar.
    pub menu_bar_height: f32,
    /// Horizontal padding inside the in-window fallback menu bar.
    pub menu_bar_padding_x: f32,
    /// Vertical padding inside the in-window fallback menu bar.
    pub menu_bar_padding_y: f32,
    /// Gap between top-level menu buttons.
    pub menu_bar_gap: f32,
    /// Minimum width of each top-level menu button.
    pub menu_bar_button_width: f32,
    /// Height of each top-level menu button.
    pub menu_bar_button_height: f32,
    /// Horizontal padding inside top-level menu buttons.
    pub menu_bar_button_padding_x: f32,
    /// Corner radius of top-level menu buttons.
    pub menu_bar_button_radius: f32,
    /// Text size used by menu labels.
    pub menu_text_size: f32,
    /// Top position of the in-window fallback floating menu panel.
    pub menu_panel_top: f32,
    /// Width of the in-window fallback floating menu panel.
    pub menu_panel_width: f32,
    /// Padding inside floating menu panels.
    pub menu_panel_padding: f32,
    /// Gap between items inside floating menu panels.
    pub menu_panel_gap: f32,
    /// Corner radius of floating menu panels.
    pub menu_panel_radius: f32,
    /// Height of each floating menu item.
    pub menu_item_height: f32,
    /// Horizontal padding inside floating menu items.
    pub menu_item_padding_x: f32,
    /// Corner radius of floating menu items.
    pub menu_item_radius: f32,
    /// Horizontal margin around menu separators.
    pub menu_separator_margin_x: f32,
    /// Vertical margin around menu separators.
    pub menu_separator_margin_y: f32,
    /// Height of menu separators.
    pub menu_separator_height: f32,
    /// Width of the root insert context menu panel.
    pub context_menu_panel_width: f32,
    /// Width of the insert-submenu panel.
    pub context_menu_submenu_width: f32,
    /// Horizontal gap between a context menu and its submenu.
    pub context_menu_submenu_gap: f32,
    /// Width of the table-axis context menu panel.
    pub context_menu_axis_panel_width: f32,
    /// Corner radius of select dropdown trigger buttons.
    pub select_trigger_radius: f32,
    /// Corner radius of select dropdown floating panels.
    pub select_panel_radius: f32,
    /// Corner radius of select dropdown option items.
    pub select_option_radius: f32,
    /// Corner radius of stepper containers and buttons.
    pub stepper_radius: f32,
    /// Maximum width of the table-insert dialog.
    pub table_insert_dialog_width: f32,
    /// Gap between table-insert stepper label and controls.
    pub table_insert_stepper_gap: f32,
    /// Size of table-insert stepper buttons.
    pub table_insert_stepper_button_size: f32,
    /// Minimum width of the table-insert stepper value pill.
    pub table_insert_stepper_value_min_width: f32,
    /// Horizontal padding inside the table-insert stepper value pill.
    pub table_insert_stepper_value_padding_x: f32,
    /// Corner radius of explorer file-tree item rows.
    pub tree_item_radius: f32,
    /// Corner radius of outline tree node item rows.
    pub outline_node_radius: f32,
    /// Corner radius of outline and preview badges.
    pub badge_radius: f32,
    /// Corner radius of settings section cards.
    pub section_card_radius: f32,
    /// Corner radius of settings item rows.
    pub settings_row_radius: f32,
    /// Height of the area top bar.
    pub topbar_height: f32,
    /// Height of the status bar.
    pub bottombar_height: f32,
    /// Horizontal padding inside the status bar.
    pub bottombar_padding_x: f32,
    /// Gap between items in the status bar.
    pub bottombar_item_gap: f32,
    /// Font size for status bar text.
    pub bottombar_text_size: f32,
    pub panel_tile_gap: f32,
    /// Uniform inset of a pane inside its split panel.
    pub pane_gap: f32,
    pub panel_tile_radius: f32,
}

