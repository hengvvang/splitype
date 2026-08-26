//! Strong Newtype wrappers for document coordinates, offsets, and scroll anchors.

use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Byte offset within the Markdown source text.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceByteOffset(pub usize);

impl fmt::Debug for SourceByteOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SourceByte({})", self.0)
    }
}

impl Add<usize> for SourceByteOffset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for SourceByteOffset {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

/// Character/grapheme offset within the plain/visual text presentation.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisualCharOffset(pub usize);

impl fmt::Debug for VisualCharOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VisualChar({})", self.0)
    }
}

impl Add<usize> for VisualCharOffset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for VisualCharOffset {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

/// Vertical pixel position relative to document top.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct PixelY(pub f32);

impl fmt::Debug for PixelY {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PixelY({:.1}px)", self.0)
    }
}

impl Add<f32> for PixelY {
    type Output = Self;
    fn add(self, rhs: f32) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<f32> for PixelY {
    fn add_assign(&mut self, rhs: f32) {
        self.0 += rhs;
    }
}

impl Sub<f32> for PixelY {
    type Output = Self;
    fn sub(self, rhs: f32) -> Self {
        Self(self.0 - rhs)
    }
}

impl SubAssign<f32> for PixelY {
    fn sub_assign(&mut self, rhs: f32) {
        self.0 -= rhs;
    }
}

/// Measured or estimated pixel height of a block or line.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct PixelHeight(pub f32);

impl fmt::Debug for PixelHeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PixelHeight({:.1}px)", self.0)
    }
}

impl Add for PixelHeight {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for PixelHeight {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

/// Anchor for jitter-free scroll pinning during dynamic layout changes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollAnchor {
    /// 0-based index of the anchor block.
    pub block_index: usize,
    /// Vertical offset within the anchor block.
    pub offset_in_block: PixelY,
}
