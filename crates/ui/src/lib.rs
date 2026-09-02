//! Reusable UI components — small, business-free building blocks.
//!
//! Components here consume `editor_contracts` data contracts (outline nodes,
//! search state) plus `theme`, `config`, `splitter`, and gpui, so any view
//! can reuse them without depending on the editor family.

pub mod bottombar;
pub mod button;
pub mod chrome;
pub mod custom_titlebar;
pub mod dialog;
pub mod empty_state;
pub mod menu_bar;
pub mod menu_item;
pub mod outline_hud;
pub mod popover;
pub mod search_input;
pub mod search_panel;
pub mod section;
pub mod select;
pub mod settings_form;
pub mod split;
pub mod stepper;
pub mod switch;
pub mod tab;
pub mod table_matrix_picker;
pub mod topbar;

pub use chrome::panel_topbar_icon;
pub use outline_hud::render_floating_outline_hud;
pub use search_panel::render_search_panel_overlay;
pub use settings_form::{
    NumberFieldProps, SearchableFontPickerProps, SettingsClickHandler, SettingsKeyHandler,
    SettingsOptionHandler, SettingsSearchHandler, make_row, make_row_with_reset, make_section,
    render_number_field, render_searchable_font_picker,
};
pub use split::{OverlayStyle, border_menu_style, render_corner_drag_preview};
