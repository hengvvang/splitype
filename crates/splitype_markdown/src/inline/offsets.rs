//! Bidirectional offset map and edit result types for inline text.
//!
//! `SourceOffsetMap` maps between plain text offsets (the fragment tree, no
//! markers) and generated source positions (the serialized Markdown). The
//! "plain" side is the tree text; the "source" side is the Markdown text
//! produced by serialization. `InlineEditResult` captures the new tree and
//! the offset mapping after an edit operation.

use std::fmt;
use std::ops::{Add, Deref, Range, Sub};

use crate::inline::text::BlockText;

/// UTF-8 offset within a rendered display string with active delimiters projected.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisplayOffset(pub usize);

impl DisplayOffset {
    pub const ZERO: Self = Self(0);
    pub fn new(val: usize) -> Self {
        Self(val)
    }
    pub fn get(self) -> usize {
        self.0
    }
}

impl Deref for DisplayOffset {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<usize> for DisplayOffset {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<DisplayOffset> for usize {
    fn from(v: DisplayOffset) -> Self {
        v.0
    }
}
impl fmt::Display for DisplayOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Add<usize> for DisplayOffset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}
impl Sub<usize> for DisplayOffset {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self(self.0.saturating_sub(rhs))
    }
}

/// UTF-8 offset within unformatted/plain `BlockText` (without active syntax delimiters).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlainOffset(pub usize);

impl PlainOffset {
    pub const ZERO: Self = Self(0);
    pub fn new(val: usize) -> Self {
        Self(val)
    }
    pub fn get(self) -> usize {
        self.0
    }
}

impl Deref for PlainOffset {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<usize> for PlainOffset {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<PlainOffset> for usize {
    fn from(v: PlainOffset) -> Self {
        v.0
    }
}
impl fmt::Display for PlainOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Add<usize> for PlainOffset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}
impl Sub<usize> for PlainOffset {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self(self.0.saturating_sub(rhs))
    }
}

/// UTF-8 offset within serialized source Markdown documents.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceOffset(pub usize);

impl SourceOffset {
    pub const ZERO: Self = Self(0);
    pub fn new(val: usize) -> Self {
        Self(val)
    }
    pub fn get(self) -> usize {
        self.0
    }
}

impl Deref for SourceOffset {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<usize> for SourceOffset {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<SourceOffset> for usize {
    fn from(v: SourceOffset) -> Self {
        v.0
    }
}
impl fmt::Display for SourceOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Add<usize> for SourceOffset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}
impl Sub<usize> for SourceOffset {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self(self.0.saturating_sub(rhs))
    }
}

/// UTF-16 offset used by IME subsystems and GPUI's `EntityInputHandler`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Utf16Offset(pub usize);

impl Utf16Offset {
    pub const ZERO: Self = Self(0);
    pub fn new(val: usize) -> Self {
        Self(val)
    }
    pub fn get(self) -> usize {
        self.0
    }
}

impl Deref for Utf16Offset {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<usize> for Utf16Offset {
    fn from(v: usize) -> Self {
        Self(v)
    }
}
impl From<Utf16Offset> for usize {
    fn from(v: Utf16Offset) -> Self {
        v.0
    }
}
impl fmt::Display for Utf16Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Add<usize> for Utf16Offset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}
impl Sub<usize> for Utf16Offset {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self(self.0.saturating_sub(rhs))
    }
}

/// Bidirectional offset map between plain inline text and source Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceOffsetMap {
    pub(crate) source: String,
    pub(crate) plain_to_source: Vec<usize>,
    pub(crate) source_to_plain: Vec<usize>,
}

impl SourceOffsetMap {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn plain_to_source(&self, offset: PlainOffset) -> SourceOffset {
        SourceOffset(self.plain_to_source_offset(offset.0))
    }

    pub fn plain_to_source_span(&self, range: Range<PlainOffset>) -> Range<SourceOffset> {
        self.plain_to_source(range.start)..self.plain_to_source(range.end)
    }

    pub fn source_to_plain(&self, offset: SourceOffset) -> PlainOffset {
        PlainOffset(self.source_to_plain_offset(offset.0))
    }

    pub fn plain_to_source_offset(&self, offset: usize) -> usize {
        self.plain_to_source
            .get(offset.min(self.plain_to_source.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub fn source_to_plain_offset(&self, offset: usize) -> usize {
        self.source_to_plain
            .get(offset.min(self.source_to_plain.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub fn source_to_plain_range(&self, range: Range<usize>) -> Range<usize> {
        self.source_to_plain_offset(range.start)..self.source_to_plain_offset(range.end)
    }
}

/// Result of a text replacement operation, containing the normalized tree and
/// a mapping from pre-edit text offsets to post-edit tree offsets.
#[derive(Clone, Debug)]
pub struct InlineEditResult {
    pub tree: BlockText,
    pub(crate) visible_to_normalized: Vec<usize>,
}

impl InlineEditResult {
    pub fn map_plain_offset(&self, offset: PlainOffset) -> PlainOffset {
        PlainOffset(self.map_offset(offset.0))
    }

    pub fn map_offset(&self, offset: usize) -> usize {
        self.visible_to_normalized
            .get(offset.min(self.visible_to_normalized.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub fn map_range(&self, range: &Range<usize>) -> Range<usize> {
        self.map_offset(range.start)..self.map_offset(range.end)
    }
}

/// Consolidated IME and UTF-16 / UTF-8 offset conversion utilities.
#[derive(Copy, Clone, Debug)]
pub struct ImeConverter;

impl ImeConverter {
    pub fn utf16_to_utf8_in(text: &str, utf16_offset: usize) -> usize {
        let mut utf16_count = 0;
        let mut utf8_offset = 0;

        for ch in text.chars() {
            if utf16_count >= utf16_offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    pub fn utf8_to_utf16_in(text: &str, utf8_offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in text.chars() {
            if utf8_count >= utf8_offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    pub fn utf16_range_to_utf8_in(text: &str, range_utf16: &Range<usize>) -> Range<usize> {
        Self::utf16_to_utf8_in(text, range_utf16.start)
            ..Self::utf16_to_utf8_in(text, range_utf16.end)
    }

    pub fn utf8_range_to_utf16_in(text: &str, range: &Range<usize>) -> Range<usize> {
        Self::utf8_to_utf16_in(text, range.start)..Self::utf8_to_utf16_in(text, range.end)
    }

    pub fn utf16_to_display_offset(text: &str, offset: Utf16Offset) -> DisplayOffset {
        DisplayOffset(Self::utf16_to_utf8_in(text, offset.0))
    }

    pub fn display_to_utf16_offset(text: &str, offset: DisplayOffset) -> Utf16Offset {
        Utf16Offset(Self::utf8_to_utf16_in(text, offset.0))
    }
}


