//! Points on the 2D character grid.

use num::traits::Zero;
use std::cmp::{max, min, Ordering};
use std::ops::{Add, Div, Mul, Sub};
use XY;

/// Simple 2D size, in cells.
///
/// Note: due to a bug in rustdoc ([#32077]), the documentation for `Vec2` is
/// currently shown on the [`XY`] page.
///
/// [#32077]: https://github.com/rust-lang/rust/issues/32077
/// [`XY`]: ../struct.XY.html
pub type Vec2 = XY<usize>;

impl<T: PartialOrd> PartialOrd for XY<T> {
    /// `a < b` <=> `a.x < b.x && a.y < b.y`
    fn partial_cmp(&self, other: &XY<T>) -> Option<Ordering> {
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

impl XY<usize> {
    /// Saturating subtraction. Computes `self - other`, saturating at 0.
    ///
    /// Never panics.
    pub fn saturating_sub<O: Into<Self>>(&self, other: O) -> Self {
        let other = other.into();
        self.zip_map(other, usize::saturating_sub)
    }

    /// Saturating addition with a signed vec.
    ///
    /// Any coordinates saturates to 0.
    pub fn saturating_add<O: Into<XY<isize>>>(&self, other: O) -> Self {
        let other = other.into();

        self.zip_map(other, |s, o| {
            if o > 0 {
                s + o as usize
            } else {
                s.saturating_sub((-o) as usize)
            }
        })
    }

    /// Checked subtraction. Computes `self - other` if possible.
    ///
    /// Returns `None` if `self.x < other.x || self.y < other.y`.
    ///
    /// Never panics.
    pub fn checked_sub<O: Into<Self>>(&self, other: O) -> Option<Self> {
        let other = other.into();
        if self.fits(other) {
            Some(*self - other)
        } else {
            None
        }
    }

    /// Returns a `XY<isize>` from `self`.
    pub fn signed(self) -> XY<isize> {
        self.into()
    }
}

impl<T: Ord> XY<T> {
    /// Returns `true` if `self` could fit inside `other`.
    ///
    /// Shortcut for `self.x <= other.x && self.y <= other.y`.
    ///
    /// If this returns `true`, then `other - self` will not underflow.
    pub fn fits_in<O: Into<Self>>(&self, other: O) -> bool {
        let other = other.into();
        self.x <= other.x && self.y <= other.y
    }

    /// Returns `true` if `other` could fit inside `self`.
    ///
    /// Shortcut for `self.x >= other.x && self.y >= other.y`.
    ///
    /// If this returns `true`, then `self - other` will not underflow.
    pub fn fits<O: Into<Self>>(&self, other: O) -> bool {
        let other = other.into();
        self.x >= other.x && self.y >= other.y
    }

    /// Returns a new Vec2 that is a maximum per coordinate.
    pub fn max<A: Into<XY<T>>, B: Into<XY<T>>>(a: A, b: B) -> Self {
        let a = a.into();
        let b = b.into();
        a.zip_map(b, max)
    }

    /// Returns a new Vec2 that is no larger than any input in both dimensions.
    pub fn min<A: Into<XY<T>>, B: Into<XY<T>>>(a: A, b: B) -> Self {
        let a = a.into();
        let b = b.into();
        a.zip_map(b, min)
    }

    /// Returns the minimum of `self` and `other`.
    pub fn or_min<O: Into<XY<T>>>(self, other: O) -> Self {
        Self::min(self, other)
    }

    /// Returns the maximum of `self` and `other`.
    pub fn or_max<O: Into<XY<T>>>(self, other: O) -> Self {
        Self::max(self, other)
    }
}

impl<T: Ord + Add<Output = T> + Clone> XY<T> {
    /// Returns (max(self.x,other.x), self.y+other.y)
    pub fn stack_vertical(&self, other: &Self) -> Self {
        Self::new(
            max(self.x.clone(), other.x.clone()),
            self.y.clone() + other.y.clone(),
        )
    }

    /// Returns (self.x+other.x, max(self.y,other.y))
    pub fn stack_horizontal(&self, other: &Self) -> Self {
        Self::new(
            self.x.clone() + other.x.clone(),
            max(self.y.clone(), other.y.clone()),
        )
    }

    /// Returns `true` if `self` fits in the given rectangle.
    pub fn fits_in_rect<O1, O2>(&self, top_left: O1, size: O2) -> bool
    where
        O1: Into<Self>,
        O2: Into<Self>,
    {
        let top_left = top_left.into();
        self.fits(top_left.clone()) && self < &(top_left + size)
    }
}

impl<T: Zero + Clone> XY<T> {
    /// Returns a vector with the X component of self, and y=0.
    pub fn keep_x(&self) -> Self {
        Self::new(self.x.clone(), T::zero())
    }

    /// Returns a vector with the Y component of self, and x=0.
    pub fn keep_y(&self) -> Self {
        Self::new(T::zero(), self.y.clone())
    }

    /// Alias for `Self::new(0,0)`.
    pub fn zero() -> Self {
        Self::new(T::zero(), T::zero())
    }
}

// Anything that can become XY<usize> can also become XY<isize>
impl<T> From<T> for XY<isize>
where
    T: Into<XY<usize>>,
{
    fn from(t: T) -> Self {
        let other = t.into();
        Self::new(other.x as isize, other.y as isize)
    }
}

impl From<(i32, i32)> for XY<usize> {
    fn from((x, y): (i32, i32)) -> Self {
        (x as usize, y as usize).into()
    }
}

impl From<(u32, u32)> for XY<usize> {
    fn from((x, y): (u32, u32)) -> Self {
        (x as usize, y as usize).into()
    }
}

impl From<(u8, u8)> for XY<usize> {
    fn from((x, y): (u8, u8)) -> Self {
        (x as usize, y as usize).into()
    }
}

impl From<(u16, u16)> for XY<usize> {
    fn from((x, y): (u16, u16)) -> Self {
        (x as usize, y as usize).into()
    }
}

// Allow xy + (into xy)
impl<T, O> Add<O> for XY<T>
where
    T: Add<Output = T>,
    O: Into<XY<T>>,
{
    type Output = Self;

    fn add(self, other: O) -> Self {
        self.zip_map(other.into(), Add::add)
    }
}

impl<T, O> Sub<O> for XY<T>
where
    T: Sub<Output = T>,
    O: Into<XY<T>>,
{
    type Output = Self;

    fn sub(self, other: O) -> Self {
        self.zip_map(other.into(), Sub::sub)
    }
}

impl<T: Clone + Div<Output = T>> Div<T> for XY<T> {
    type Output = Self;

    fn div(self, other: T) -> Self {
        self.map(|s| s / other.clone())
    }
}

impl Mul<usize> for XY<usize> {
    type Output = Vec2;

    fn mul(self, other: usize) -> Vec2 {
        self.map(|s| s * other)
    }
}

#[cfg(test)]
mod tests {
    use super::Vec2;

    #[test]
    fn test_from() {
        let vi32 = Vec2::from((4i32, 5i32));
        let vu32 = Vec2::from((4u32, 5u32));

        let vusize = Vec2::from((4usize, 5usize));
        let vvec = Vec2::from(Vec2::new(4, 5));

        assert_eq!(vi32 - vu32, vusize - vvec);
    }
}
