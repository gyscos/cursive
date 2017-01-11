use direction::Orientation;

use std::iter;

/// A generic structure with a value for each axis.
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct XY<T> {
    /// X-axis value
    pub x: T,
    /// Y-axis value
    pub y: T,
}

impl<T> XY<T> {
    /// Creates a new `XY` from the given values.
    pub fn new(x: T, y: T) -> Self {
        XY { x: x, y: y }
    }

    /// Creates a new `XY` by applying `f` to `x` and `y`.
    pub fn map<U, F: Fn(T) -> U>(self, f: F) -> XY<U> {
        XY::new(f(self.x), f(self.y))
    }

    /// Destructure self into a pair.
    pub fn pair(self) -> (T, T) {
        (self.x, self.y)
    }

    /// Return a `XY` with references to this one's values.
    pub fn as_ref(&self) -> XY<&T> {
        XY::new(&self.x, &self.y)
    }

    /// Creates an iterator that returns references to `x`, then `y`.
    pub fn iter(&self) -> iter::Chain<iter::Once<&T>, iter::Once<&T>> {
        iter::once(&self.x).chain(iter::once(&self.y))
    }

    /// Returns a reference to the value on the given axis.
    pub fn get(&self, o: Orientation) -> &T {
        match o {
            Orientation::Horizontal => &self.x,
            Orientation::Vertical => &self.y,
        }
    }

    /// Returns a new `XY` of tuples made by zipping `self` and `other`.
    pub fn zip<U>(self, other: XY<U>) -> XY<(T, U)> {
        XY::new((self.x, other.x), (self.y, other.y))
    }

    /// Returns a new `XY` by calling `f` on `self` and `other` for each axis.
    pub fn zip_map<U, V, F: Fn(T, U) -> V>(self, other: XY<U>, f: F) -> XY<V> {
        XY::new(f(self.x, other.x), f(self.y, other.y))
    }
}

impl <T: Clone> XY<T> {
    /// Returns a new `XY` with the axis `o` set to `value`.
    pub fn with_axis(&self, o: Orientation, value: T) -> Self {
        let mut new = self.clone();
        *o.get_ref(&mut new) = value;
        new
    }

    /// Returns a new `XY` with the axis `o` set to the value from `other`.
    pub fn with_axis_from(&self, o: Orientation, other: &Self) -> Self {
        let mut new = self.clone();
        new.set_axis_from(o, other);
        new
    }

    /// Sets the axis `o` on `self` to the value from `other`.
    pub fn set_axis_from(&mut self, o: Orientation, other: &Self) {
        *o.get_ref(self) = o.get(other);
    }
}

impl<T> XY<Option<T>> {
    /// Returns a new `XY` by calling `unwrap_or` on each axis.
    pub fn unwrap_or(self, other: XY<T>) -> XY<T> {
        self.zip_map(other, |s, o| s.unwrap_or(o))
    }
}

impl XY<bool> {
    /// Returns `true` if any of `x` or `y` is `true`.
    pub fn any(&self) -> bool {
        self.x || self.y
    }

    /// Returns `true` if both `x` and `y` are `true`.
    pub fn both(&self) -> bool {
        self.x && self.y
    }
}

impl<T: Copy> XY<T> {
    /// Creates a `XY` with both `x` and `y` set to `value`.
    pub fn both_from(value: T) -> Self {
        XY::new(value, value)
    }
}

impl<T> From<(T, T)> for XY<T> {
    fn from((x, y): (T, T)) -> Self {
        XY::new(x, y)
    }
}
