//! Inline text model: styled fragment trees, Markdown parsing/serialization,
//! and render caches.

pub mod footnote;
pub mod html;
pub mod latex;
pub mod link;
pub mod markdown;
pub mod offsets;
pub mod render_cache;
pub mod serialize;
pub mod style;
pub mod text;

pub use offsets::ImeConverter;
