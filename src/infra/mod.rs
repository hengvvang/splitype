//! System infrastructure — capabilities independent of document content.
//!
//! Network transport and update checks (`net`), configuration persistence
//! (`config`), and localization (`i18n`). Consumed by every layer; depends
//! on nothing above `model`.

pub(crate) mod config;
pub(crate) mod i18n;
pub(crate) mod net;
