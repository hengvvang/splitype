//! Block-level content models — one file per Markdown block syntax feature
//! (table, HTML block, display math, Mermaid fence, standalone image,
//! link reference definition, footnote definition), each bundling its data
//! types with their parse and serialize helpers. Inline text lives in
//! `crate::markdown::inline`; the parsing contract types live in `crate::markdown::parse`.

pub mod callout;
pub mod footnote;
pub mod html;
pub mod image;
pub mod link;
pub mod math;
pub mod mermaid;
pub mod table;

pub use callout::CalloutKind;
