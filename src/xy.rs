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

    /// Destructure self into a pair.
    pub fn pair(self) -> (T, T) {
        (self.x, self.y)
    }

    /// Return a XY with references to this one's values.
    pub fn as_ref(&self) -> XY<&T> {
        XY::new(&self.x, &self.y)
    }

    /// Creates an iterator that returns references to `x`, then `y`.
    pub fn iter(&self) -> iter::Chain<iter::Once<&T>, iter::Once<&T>> {
        iter::once(&self.x).chain(iter::once(&self.y))
    }
}

impl<T: Copy> XY<T> {
    /// Creates a `XY` with both `x` and `y` set to `value`.
    pub fn both(value: T) -> Self {
        XY::new(value, value)
    }
}

impl<T> From<(T, T)> for XY<T> {
    fn from((x, y): (T, T)) -> Self {
        XY::new(x, y)
    }
}
