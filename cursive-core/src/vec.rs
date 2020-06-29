//! Points on the 2D character grid.

use std::cmp::{max, min, Ordering};
use std::ops::{Add, Div, Mul, Sub};

use num::traits::Zero;

use crate::div;
use crate::XY;

/// Simple 2D size, in cells.
///
/// Note: due to a bug in rustdoc ([#32077]), the documentation for `Vec2` is
/// currently shown on the [`XY`] page.
///
/// [#32077]: https://github.com/rust-lang/rust/issues/32077
/// [`XY`]: crate::XY
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
    /// Returns a `Vec2` with `usize::max_value()` in each axis.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// assert!(Vec2::new(9999, 9999) < Vec2::max_value());
    /// ```
    pub fn max_value() -> Self {
        Self::new(usize::max_value(), usize::max_value())
    }

    /// Saturating subtraction. Computes `self - other`, saturating at 0.
    ///
    /// Never panics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let u = Vec2::new(1, 2);
    /// let v = Vec2::new(2, 1);
    /// assert_eq!(u.saturating_sub(v), Vec2::new(0, 1));
    /// ```
    pub fn saturating_sub<O: Into<Self>>(&self, other: O) -> Self {
        let other = other.into();
        self.zip_map(other, usize::saturating_sub)
    }

    /// Saturating addition with a signed vec.
    ///
    /// Any coordinates saturates to 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// # use cursive_core::XY;
    /// let u = Vec2::new(1, 2);
    /// let v = XY::<isize>::new(-2, 1);
    /// assert_eq!(u.saturating_add(v), Vec2::new(0, 3));
    /// ```
    pub fn saturating_add<O: Into<XY<isize>>>(&self, other: O) -> Self {
        let other = other.into();

        self.zip_map(other, |s, o| {
            if o > 0 {
                s.saturating_add(o as usize)
            } else {
                s.saturating_sub((-o) as usize)
            }
        })
    }

    /// Checked addition with a signed vec.
    ///
    /// Will return `None` if any coordinates exceeds bounds.
    pub fn checked_add<O: Into<XY<isize>>>(&self, other: O) -> Option<Self> {
        let other = other.into();
        self.zip_map(other, |s, o| {
            if o > 0 {
                s.checked_add(o as usize)
            } else {
                s.checked_sub((-o) as usize)
            }
        })
        .both()
    }

    /// Term-by-term integer division that rounds up.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let u = Vec2::new(1, 6);
    /// let v = Vec2::new(2, 3);
    /// assert_eq!(u.div_up(v), Vec2::new(1, 2));
    /// ```
    pub fn div_up<O>(&self, other: O) -> Self
    where
        O: Into<Self>,
    {
        self.zip_map(other.into(), div::div_up)
    }

    /// Checked subtraction. Computes `self - other` if possible.
    ///
    /// Returns `None` if `self.x < other.x || self.y < other.y`.
    ///
    /// Never panics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let xy = Vec2::new(1, 2);
    /// assert_eq!(xy.checked_sub((1, 1)), Some(Vec2::new(0, 1)));
    /// assert_eq!(xy.checked_sub((2, 2)), None);
    /// ```
    pub fn checked_sub<O: Into<Self>>(&self, other: O) -> Option<Self> {
        let other = other.into();
        if self.fits(other) {
            Some(*self - other)
        } else {
            None
        }
    }

    /// Returns a `XY<isize>` from `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// # use cursive_core::XY;
    /// let v: XY<isize> = Vec2::new(1, 2).signed().map(|i| i - 5);
    /// assert_eq!(v, XY::new(-4, -3));
    ///
    /// let u = Vec2::new(3, 4);
    /// assert_eq!(u.saturating_add(v), Vec2::new(0, 1));
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let v = Vec2::new(1, 2);
    /// assert!(v.fits_in((1, 2)));
    /// assert!(v.fits_in((3, 3)));
    /// assert!(!v.fits_in((2, 1)));
    /// ```
    pub fn fits_in<O: Into<Self>>(&self, other: O) -> bool {
        let other = other.into();
        self.x <= other.x && self.y <= other.y
    }

    /// Returns `true` if `other` could fit inside `self`.
    ///
    /// Shortcut for `self.x >= other.x && self.y >= other.y`.
    ///
    /// If this returns `true`, then `self - other` will not underflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let v = Vec2::new(1, 2);
    /// assert!(v.fits((1, 2)));
    /// assert!(v.fits((0, 0)));
    /// assert!(!v.fits((2, 1)));
    /// ```
    pub fn fits<O: Into<Self>>(&self, other: O) -> bool {
        let other = other.into();
        self.x >= other.x && self.y >= other.y
    }

    /// Returns `true` if `other` is strictly less than `self` in each axis.
    pub fn strictly_lt<O: Into<Self>>(&self, other: O) -> bool {
        let other = other.into();
        self < &other
    }

    /// Returns `true` if `other` is strictly greater than `self` in each axis.
    pub fn strictly_gt<O: Into<Self>>(&self, other: O) -> bool {
        let other = other.into();
        self > &other
    }

    /// Returns a new Vec2 that is a maximum per coordinate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// assert_eq!(Vec2::max((1, 2), (3, 1)), Vec2::new(3, 2));
    /// ```
    pub fn max<A: Into<XY<T>>, B: Into<XY<T>>>(a: A, b: B) -> Self {
        let a = a.into();
        let b = b.into();
        a.zip_map(b, max)
    }

    /// Returns a new Vec2 that is no larger than any input in both dimensions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// assert_eq!(Vec2::min((1, 2), (3, 1)), Vec2::new(1, 1));
    /// ```
    pub fn min<A: Into<XY<T>>, B: Into<XY<T>>>(a: A, b: B) -> Self {
        let a = a.into();
        let b = b.into();
        a.zip_map(b, min)
    }

    /// Returns the minimum of `self` and `other`.
    ///
    /// This is equivalent to `Vec2::min(self, other)`.
    pub fn or_min<O: Into<XY<T>>>(self, other: O) -> Self {
        Self::min(self, other)
    }

    /// Returns the maximum of `self` and `other`.
    ///
    /// This is equivalent to `Vec2::max(self, other)`.
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

impl<T: Add> XY<T> {
    /// Returns `self.x + self.y`.
    pub fn sum(self) -> T::Output {
        self.fold(std::ops::Add::add)
    }
}

impl<T: Mul> XY<T> {
    /// Returns `self.x * self.y`
    pub fn product(self) -> T::Output {
        self.fold(std::ops::Mul::mul)
    }
}

impl<T: Zero + Clone> XY<T> {
    /// Returns a vector with the X component of self, and y=0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// assert_eq!(xy.keep_x(), XY::new(1, 0));
    /// ```
    pub fn keep_x(&self) -> Self {
        Self::new(self.x.clone(), T::zero())
    }

    /// Returns a vector with the Y component of self, and x=0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// assert_eq!(xy.keep_y(), XY::new(0, 2));
    /// ```
    pub fn keep_y(&self) -> Self {
        Self::new(T::zero(), self.y.clone())
    }

    /// Alias for `Self::new(0,0)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// assert_eq!(Vec2::zero(), Vec2::new(0, 0));
    /// ```
    pub fn zero() -> Self {
        Self::new(T::zero(), T::zero())
    }
}

impl<'a, T> From<&'a XY<T>> for XY<T>
where
    T: Clone,
{
    /// Clone a XY
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(String::from("a"), String::from("ab"));
    /// assert_eq!(XY::from(&xy), xy);
    /// ```
    fn from(t: &'a XY<T>) -> Self {
        t.clone()
    }
}

// Anything that can become XY<usize> can also become XY<isize>
impl<T> From<T> for XY<isize>
where
    T: Into<XY<usize>>,
{
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// # use cursive_core::Vec2;
    /// let u = Vec2::new(1, 2);
    /// let v: XY<isize> = XY::from(u);
    /// assert_eq!(v, XY::new(1, 2));
    /// ```
    fn from(t: T) -> Self {
        let other = t.into();
        Self::new(other.x as isize, other.y as isize)
    }
}

impl From<(i32, i32)> for XY<usize> {
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy: XY<isize> = XY::from((-1i32, -2i32));
    /// assert_eq!(xy, XY::new(-1, -2));
    /// ```
    fn from((x, y): (i32, i32)) -> Self {
        (x as usize, y as usize).into()
    }
}

impl From<(u32, u32)> for XY<usize> {
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let v = Vec2::from((1u32, 2u32));
    /// assert_eq!(v, Vec2::new(1, 2));
    /// ```
    fn from((x, y): (u32, u32)) -> Self {
        (x as usize, y as usize).into()
    }
}

impl From<(u8, u8)> for XY<usize> {
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let v = Vec2::from((1u8, 2u8));
    /// assert_eq!(v, Vec2::new(1, 2));
    /// ```
    fn from((x, y): (u8, u8)) -> Self {
        (x as usize, y as usize).into()
    }
}

impl From<(u16, u16)> for XY<usize> {
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let v = Vec2::from((1u16, 2u16));
    /// assert_eq!(v, Vec2::new(1, 2));
    /// ```
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

    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// assert_eq!(xy + (2, 3), XY::new(3, 5));
    /// ```
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

    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// assert_eq!(xy - (1, 0), XY::new(0, 2));
    /// ```
    fn sub(self, other: O) -> Self {
        self.zip_map(other.into(), Sub::sub)
    }
}

impl<T: Clone + Div<Output = T>> Div<T> for XY<T> {
    type Output = Self;

    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 4);
    /// assert_eq!(xy / 2, XY::new(0, 2));
    /// ```
    fn div(self, other: T) -> Self {
        self.map(|s| s / other.clone())
    }
}

impl Mul<usize> for XY<usize> {
    type Output = Vec2;

    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Vec2;
    /// let v = Vec2::new(1, 2);
    /// assert_eq!(v * 2, Vec2::new(2, 4));
    /// ```
    fn mul(self, other: usize) -> Vec2 {
        self.map(|s| s * other)
    }
}

impl<T> Mul<XY<T>> for XY<T>
where
    T: Mul<T>,
{
    type Output = XY<T::Output>;

    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let u = XY::new(1, 2);
    /// let v = XY::new(2, 3);
    /// assert_eq!(u * v, XY::new(2, 6));
    /// ```
    fn mul(self, other: XY<T>) -> Self::Output {
        self.zip_map(other, |s, o| s * o)
    }
}
impl<T> Div<XY<T>> for XY<T>
where
    T: Div<T>,
{
    type Output = XY<T::Output>;

    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let u = XY::new(2, 3);
    /// let v = XY::new(1, 2);
    /// assert_eq!(u / v, XY::new(2, 1));
    /// ```
    fn div(self, other: XY<T>) -> Self::Output {
        self.zip_map(other, |s, o| s / o)
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
