use crate::vec::Vec2;
use std::ops::{Add, Div, Mul, Sub};

/// Four values representing each direction.
#[derive(Clone, Copy)]
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
    /// Creates a new Margins.
    pub fn new(left: usize, right: usize, top: usize, bottom: usize) -> Self {
        Margins {
            left,
            right,
            top,
            bottom,
        }
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

impl From<(usize, usize, usize, usize)> for Margins {
    fn from(
        (left, right, top, bottom): (usize, usize, usize, usize),
    ) -> Margins {
        Margins::new(left, right, top, bottom)
    }
}

impl From<(i32, i32, i32, i32)> for Margins {
    fn from((left, right, top, bottom): (i32, i32, i32, i32)) -> Margins {
        (left as usize, right as usize, top as usize, bottom as usize).into()
    }
}

impl From<((i32, i32), (i32, i32))> for Margins {
    fn from(
        ((left, right), (top, bottom)): ((i32, i32), (i32, i32)),
    ) -> Margins {
        (left, right, top, bottom).into()
    }
}
impl From<((usize, usize), (usize, usize))> for Margins {
    fn from(
        ((left, right), (top, bottom)): ((usize, usize), (usize, usize)),
    ) -> Margins {
        (left, right, top, bottom).into()
    }
}

impl<T: Into<Margins>> Add<T> for Margins {
    type Output = Margins;

    fn add(self, other: T) -> Margins {
        let ov = other.into();

        Margins {
            left: self.left + ov.left,
            right: self.right + ov.right,
            top: self.top + ov.top,
            bottom: self.bottom + ov.bottom,
        }
    }
}

impl<T: Into<Margins>> Sub<T> for Margins {
    type Output = Margins;

    fn sub(self, other: T) -> Margins {
        let ov = other.into();

        Margins {
            left: self.left - ov.left,
            right: self.right - ov.right,
            top: self.top - ov.top,
            bottom: self.bottom - ov.bottom,
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
