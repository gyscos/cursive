//! Points on the 2D character grid.

use std::ops::{Add, Div, Mul, Sub};
use std::cmp::{Ordering, max, min};

/// Simple 2D size, in characters.
#[derive(Clone,Copy,PartialEq,Debug)]
pub struct Vec2 {
    /// X coordinate (column), from left to right.
    pub x: usize,
    /// Y coordinate (row), from top to bottom.
    pub y: usize,
}

impl PartialOrd for Vec2 {
    fn partial_cmp(&self, other: &Vec2) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if self.x < other.x && self.y < other.y {
            Some(Ordering::Less)
        } else if self.x > other.x && self.y > other.y {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}

impl Vec2 {
    /// Creates a new Vec2 from coordinates.
    pub fn new(x: usize, y: usize) -> Self {
        Vec2 { x: x, y: y }
    }

    /// Returns a new Vec2 that is a maximum per coordinate.
    pub fn max(a: Self, b: Self) -> Self {
        Vec2::new(max(a.x, b.x), max(a.y, b.y))
    }

    /// Returns a new Vec2 that is no larger than any input in both dimensions.
    pub fn min(a: Self, b: Self) -> Self {
        Vec2::new(min(a.x, b.x), min(a.y, b.y))
    }

    /// Returns a vector with the X component of self, and y=0.
    pub fn keep_x(&self) -> Self {
        Vec2::new(self.x, 0)
    }

    /// Returns a vector with the Y component of self, and x=0.
    pub fn keep_y(&self) -> Self {
        Vec2::new(0, self.y)
    }

    /// Alias for Vec::new(0,0).
    pub fn zero() -> Self {
        Vec2::new(0, 0)
    }

    /// Returns (max(self.x,other.x), self.y+other.y)
    pub fn stack_vertical(&self, other: &Vec2) -> Vec2 {
        Vec2::new(max(self.x, other.x), self.y + other.y)
    }

    /// Returns (self.x+other.x, max(self.y,other.y))
    pub fn stack_horizontal(&self, other: &Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, max(self.y, other.y))
    }
}

impl From<(usize, usize)> for Vec2 {
    fn from((x, y): (usize, usize)) -> Self {
        Vec2::new(x, y)
    }
}

impl From<(i32, i32)> for Vec2 {
    fn from((x, y): (i32, i32)) -> Self {
        (x as usize, y as usize).into()
    }
}

impl From<(u32, u32)> for Vec2 {
    fn from((x, y): (u32, u32)) -> Self {
        (x as usize, y as usize).into()
    }
}


impl<T: Into<Vec2>> Add<T> for Vec2 {
    type Output = Vec2;

    fn add(self, other: T) -> Vec2 {
        let ov = other.into();
        Vec2 {
            x: self.x + ov.x,
            y: self.y + ov.y,
        }
    }
}

impl<T: Into<Vec2>> Sub<T> for Vec2 {
    type Output = Vec2;

    fn sub(self, other: T) -> Vec2 {
        let ov = other.into();
        Vec2 {
            x: self.x - ov.x,
            y: self.y - ov.y,
        }
    }
}

impl Div<usize> for Vec2 {
    type Output = Vec2;

    fn div(self, other: usize) -> Vec2 {
        Vec2 {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl Mul<usize> for Vec2 {
    type Output = Vec2;

    fn mul(self, other: usize) -> Vec2 {
        Vec2 {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

/// Four values representing each direction.
#[derive(Clone,Copy)]
pub struct Vec4 {
    /// Left margin
    pub left: usize,
    /// Right margin
    pub right: usize,
    /// Top margin
    pub top: usize,
    /// Bottom margin
    pub bottom: usize,
}

impl Vec4 {
    /// Creates a new Vec4.
    pub fn new(left: usize, right: usize, top: usize, bottom: usize) -> Self {
        Vec4 {
            left: left,
            right: right,
            top: top,
            bottom: bottom,
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

impl From<(usize, usize, usize, usize)> for Vec4 {
    fn from((left, right, top, bottom): (usize, usize, usize, usize)) -> Vec4 {
        Vec4::new(left, right, top, bottom)
    }
}

impl From<(i32, i32, i32, i32)> for Vec4 {
    fn from((left, right, top, bottom): (i32, i32, i32, i32)) -> Vec4 {
        (left as usize, right as usize, top as usize, bottom as usize).into()
    }
}

impl<T: Into<Vec4>> Add<T> for Vec4 {
    type Output = Vec4;

    fn add(self, other: T) -> Vec4 {
        let ov = other.into();

        Vec4 {
            left: self.left + ov.left,
            right: self.right + ov.right,
            top: self.top + ov.top,
            bottom: self.bottom + ov.bottom,
        }
    }
}

impl<T: Into<Vec4>> Sub<T> for Vec4 {
    type Output = Vec4;

    fn sub(self, other: T) -> Vec4 {
        let ov = other.into();

        Vec4 {
            left: self.left - ov.left,
            right: self.right - ov.right,
            top: self.top - ov.top,
            bottom: self.bottom - ov.bottom,
        }
    }
}


impl Div<usize> for Vec4 {
    type Output = Vec4;

    fn div(self, other: usize) -> Vec4 {
        Vec4 {
            left: self.left / other,
            right: self.right / other,
            top: self.top / other,
            bottom: self.bottom / other,
        }
    }
}

impl Mul<usize> for Vec4 {
    type Output = Vec4;

    fn mul(self, other: usize) -> Vec4 {
        Vec4 {
            left: self.left * other,
            right: self.right * other,
            top: self.top * other,
            bottom: self.bottom * other,
        }
    }
}
