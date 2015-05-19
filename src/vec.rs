//! Points on the 2D character grid.

use std::ops::{Add, Sub, Mul, Div};
use std::cmp::{min,max};

/// Simple 2D size, in characters.
#[derive(Clone,Copy)]
pub struct Vec2 {
    /// X coordinate (column), from left to right.
    pub x: u32,
    /// Y coordinate (row), from top to bottom.
    pub y: u32,
}

impl Vec2 {
    /// Creates a new Vec2 from coordinates.
    pub fn new(x: u32, y: u32) -> Self {
        Vec2 {
            x: x,
            y: y,
        }
    }

    /// Returns a new Vec2 that is a maximum per coordinate.
    pub fn max(a: Self, b: Self) -> Self {
        Vec2::new(max(a.x, b.x), max(a.y, b.y))
    }

    /// Returns a new Vec2 that is no larger than any input in both dimensions.
    pub fn min(a: Self, b: Self) -> Self {
        Vec2::new(min(a.x, b.x), min(a.y, b.y))
    }

    pub fn keep_x(&self) -> Self {
        Vec2::new(self.x, 0)
    }

    pub fn keep_y(&self) -> Self {
        Vec2::new(0, self.y)
    }

    /// Alias for Vec::new(0,0)
    pub fn zero() -> Self {
        Vec2::new(0,0)
    }
}

/// A generic trait for converting a value into a 2D vector.
pub trait ToVec2 {
    fn to_vec2(self) -> Vec2;
}

impl ToVec2 for Vec2 {
    fn to_vec2(self) -> Vec2 {
        self
    }
}

impl ToVec2 for (u32,u32) {
    fn to_vec2(self) -> Vec2 {
        Vec2::new(self.0, self.1)
    }
}

impl ToVec2 for (usize,usize) {
    fn to_vec2(self) -> Vec2 {
        Vec2::new(self.0 as u32, self.1 as u32)
    }
}

impl <T: ToVec2> Add<T> for Vec2 {
    type Output = Vec2;

    fn add(self, other: T) -> Vec2 {
        let ov = other.to_vec2();
        Vec2 {
            x: self.x + ov.x,
            y: self.y + ov.y,
        }
    }
}

impl <T: ToVec2> Sub<T> for Vec2 {
    type Output = Vec2;

    fn sub(self, other: T) -> Vec2 {
        let ov = other.to_vec2();
        Vec2 {
            x: self.x - ov.x,
            y: self.y - ov.y,
        }
    }
}

impl Div<u32> for Vec2 {
    type Output = Vec2;

    fn div(self, other: u32) -> Vec2 {
        Vec2 {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl Mul<u32> for Vec2 {
    type Output = Vec2;

    fn mul(self, other: u32) -> Vec2 {
        Vec2 {
            x: self.x * other,
            y: self.y * other,
        }
    }
}
