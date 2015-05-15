use std::ops::{Add, Sub};
use std::cmp::min;

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

    /// Returns a new Vec2 that is no larger than any input in both dimensions.
    pub fn min(a: Vec2, b: Vec2) -> Vec2 {
        Vec2 {
            x: min(a.x, b.x),
            y: min(a.y, b.y),
        }
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

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
