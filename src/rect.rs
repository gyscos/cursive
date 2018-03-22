//! Rectangles on the 2D character grid.
use std::ops::Add;

use vec::Vec2;

/// A non-empty rectangle on the 2D grid.
///
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    top_left: Vec2,
    bottom_right: Vec2,
}

impl<T> From<T> for Rect
where
    T: Into<Vec2>,
{
    fn from(other: T) -> Self {
        // From a point, we can create a 1-by-1 rectangle.
        Self::from_size(other, (1, 1))
    }
}

impl <T> Add<T> for Rect
where T: Into<Vec2> {
    type Output = Rect;

    fn add(mut self, rhs: T) -> Self {
        self.offset(rhs);
        self
    }
}

impl Rect {
    /// Creates a new `Rect` with the given position and size.
    ///
    /// `size` must be non-zero in each axis.
    pub fn from_size<U, V>(top_left: U, size: V) -> Self
    where
        U: Into<Vec2>,
        V: Into<Vec2>,
    {
        let size = size.into();
        let top_left = top_left.into();
        assert!(size > Vec2::zero());

        let bottom_right = top_left + size.saturating_sub((1, 1));

        Self::from_corners(top_left, bottom_right)
    }

    /// Creates a new `Rect` from two corners.
    ///
    /// It can be any two opposite corners.
    pub fn from_corners<U, V>(a: U, b: V) -> Self
    where
        U: Into<Vec2>,
        V: Into<Vec2>,
    {
        let a = a.into();
        let b = b.into();

        let top_left = Vec2::min(a, b);
        let bottom_right = Vec2::max(a, b);

        Rect {
            top_left,
            bottom_right,
        }
    }

    /// Grow this rectangle if necessary to include `other`.
    pub fn expand_to<R>(&mut self, other: R)
    where
        R: Into<Rect>,
    {
        let other = other.into();

        self.top_left = self.top_left.or_min(other.top_left);
        self.bottom_right = self.bottom_right.or_max(other.bottom_right);
    }

    /// Returns a new rectangle that includes both `self` and `other`.
    pub fn expanded_to<R>(mut self, other: R) -> Self
    where
        R: Into<Rect>,
    {
        self.expand_to(other);
        self
    }

    /// Adds the given offset to this rectangle.
    pub fn offset<V>(&mut self, offset: V) where V: Into<Vec2> {
        let offset = offset.into();
        self.top_left = self.top_left + offset;
        self.bottom_right = self.bottom_right + offset;
    }

    /// Returns the size of the rectangle.
    pub fn size(self) -> Vec2 {
        self.bottom_right - self.top_left + (1, 1)
    }

    /// Returns the width of the rectangle.
    pub fn width(self) -> usize {
        self.size().x
    }

    /// Returns the height of the rectangle.
    pub fn height(self) -> usize {
        self.size().y
    }

    /// Returns the top-left corner.
    pub fn top_left(self) -> Vec2 {
        self.top_left
    }

    /// Returns the bottom-right corner.
    pub fn bottom_right(self) -> Vec2 {
        self.bottom_right
    }

    /// Returns the top-right corner.
    pub fn top_right(self) -> Vec2 {
        Vec2::new(self.right(), self.top())
    }

    /// Returns the bottom-left corner.
    pub fn bottom_left(self) -> Vec2 {
        Vec2::new(self.left(), self.bottom())
    }

    /// Returns the Y value of the top edge of the rectangle.
    pub fn top(self) -> usize {
        self.top_left.y
    }

    /// Returns the X value of the left edge of the rectangle.
    pub fn left(self) -> usize {
        self.top_left.x
    }

    /// Returns the X value of the right edge of the rectangle.
    pub fn right(self) -> usize {
        self.bottom_right.x
    }

    /// Returns the Y value of the botton edge of the rectangle.
    pub fn bottom(self) -> usize {
        self.bottom_right.y
    }

    /// Returns the surface (number of cells) covered by the rectangle.
    pub fn surface(self) -> usize {
        self.width() * self.height()
    }
}
