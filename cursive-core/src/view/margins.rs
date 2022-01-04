use crate::Vec2;
use std::ops::{Add, Div, Mul, Sub};

/// Four values representing each direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Margins {
    /// Left margin
    pub left: usize,
    /// Right margin
    pub right: usize,
    /// Top margin
    pub top: usize,
    /// Bottom margin
    pub bottom: usize,
}

impl Margins {
    /// Creates a new `Margins` object with zero margins.
    pub fn zeroes() -> Self {
        Self::lrtb(0, 0, 0, 0)
    }

    /// Creates a new `Margins` object from the Left, Right, Top, Bottom fields.
    pub fn lrtb(left: usize, right: usize, top: usize, bottom: usize) -> Self {
        Margins {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates a new `Margins` object from the Left, Top, Right, Bottom fields.
    pub fn ltrb(left_top: Vec2, right_bottom: Vec2) -> Self {
        Self::lrtb(left_top.x, right_bottom.x, left_top.y, right_bottom.y)
    }

    /// Creates a new `Margins` object from the Top, Right, Bottom, Left fields.
    pub fn trbl(top: usize, right: usize, bottom: usize, left: usize) -> Self {
        Self::lrtb(left, right, top, bottom)
    }

    /// Creates a new `Margins` object from the Left and Right fields.
    ///
    /// Top and Bottom will be 0.
    pub fn lr(left: usize, right: usize) -> Self {
        Self::lrtb(left, right, 0, 0)
    }

    /// Creates a new `Margins` object from the Top and Bottom fields.
    ///
    /// Left and Right will be 0.
    pub fn tb(top: usize, bottom: usize) -> Self {
        Self::lrtb(0, 0, top, bottom)
    }

    /// Returns left + right.
    pub fn horizontal(&self) -> usize {
        self.left + self.right
    }

    /// Returns top + bottom.
    pub fn vertical(&self) -> usize {
        self.top + self.bottom
    }

    /// Returns (left+right, top+bottom).
    pub fn combined(&self) -> Vec2 {
        Vec2::new(self.horizontal(), self.vertical())
    }

    /// Returns (left, top).
    pub fn top_left(&self) -> Vec2 {
        Vec2::new(self.left, self.top)
    }

    /// Returns (right, bottom).
    pub fn bot_right(&self) -> Vec2 {
        Vec2::new(self.right, self.bottom)
    }
}

impl Add<Margins> for Margins {
    type Output = Margins;

    fn add(self, other: Margins) -> Margins {
        Margins {
            left: self.left + other.left,
            right: self.right + other.right,
            top: self.top + other.top,
            bottom: self.bottom + other.bottom,
        }
    }
}

impl Sub<Margins> for Margins {
    type Output = Margins;

    fn sub(self, other: Margins) -> Margins {
        Margins {
            left: self.left - other.left,
            right: self.right - other.right,
            top: self.top - other.top,
            bottom: self.bottom - other.bottom,
        }
    }
}

impl Div<usize> for Margins {
    type Output = Margins;

    fn div(self, other: usize) -> Margins {
        Margins {
            left: self.left / other,
            right: self.right / other,
            top: self.top / other,
            bottom: self.bottom / other,
        }
    }
}

impl Mul<usize> for Margins {
    type Output = Margins;

    fn mul(self, other: usize) -> Margins {
        Margins {
            left: self.left * other,
            right: self.right * other,
            top: self.top * other,
            bottom: self.bottom * other,
        }
    }
}
