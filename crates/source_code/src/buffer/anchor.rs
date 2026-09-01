//! Stable location anchor in the buffer.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bias {
    Left,
    Right,
}

/// An anchor representing a position in the buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Anchor {
    pub offset: usize,
    pub bias: Bias,
}

impl Anchor {
    #[inline]
    pub const fn min() -> Self {
        Self {
            offset: 0,
            bias: Bias::Left,
        }
    }

    #[inline]
    pub const fn max() -> Self {
        Self {
            offset: usize::MAX,
            bias: Bias::Right,
        }
    }

    #[inline]
    pub const fn at(offset: usize, bias: Bias) -> Self {
        Self { offset, bias }
    }

    #[inline]
    pub fn to_offset(&self, text_len: usize) -> usize {
        self.offset.min(text_len)
    }
}
