//! presentation — the public, read-only rendering services the WYSIWYG
//! world shares with the preview pane.
//!
//! This is the *only* face the preview may consume from the editing
//! world (plan 3.2, contract 2): theme-driven style primitives and
//! self-contained render helpers that take block/markdown *data* and
//! return GPUI elements. Editing internals (document/block mutation,
//! input, history, selection) are never exposed here — module
//! visibility, review, and CI (machete + cargo tree) enforce that.
//!
//! Consumers assemble their own render trees; they only call these
//! primitives.

// List markers and quote bands (shared list/quote chrome).
pub use crate::render::list_markers::{numbered_list_marker, render_custom_bullet_marker};

// Callout accent colors and the centered content-column math.
pub use crate::render::layout::{callout_colors, centered_column_ratio, centered_column_width};

// Self-contained graphic preview boxes (LaTeX display math, Mermaid).
pub use crate::render::embedded_preview::render_graphic_preview_box;

// HTML document style tree (block-HTML rendering).
pub use crate::render::html_document::{
    HtmlComputedStyle, HtmlNodeVisualStyle, html_children_text, html_css_color_to_hsla,
    html_node_visual_style,
};

// Table column measurement.
pub use crate::table::measure::measure_table_column_layout;

// Footnote registry data contract (shared with the preview's own tree).
pub use crate::model::block::footnotes::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};

// Graphic placeholders and error cards (LaTeX/Mermaid previews).
pub use crate::render::graphic_state::{
    GraphicKind, render_empty_graphic_placeholder, render_graphic_error_card,
};


