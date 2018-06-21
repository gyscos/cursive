use direction::Orientation;
use std::iter;

/// A generic structure with a value for each axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XY<T> {
    /// X-axis value
    pub x: T,
    /// Y-axis value
    pub y: T,
}

impl<T> IntoIterator for XY<T> {
    type Item = T;
    type IntoIter = iter::Chain<iter::Once<T>, iter::Once<T>>;

    fn into_iter(self) -> Self::IntoIter {
        iter::once(self.x).chain(iter::once(self.y))
    }
}

impl<T> XY<T> {
    /// Creates a new `XY` from the given values.
    pub fn new(x: T, y: T) -> Self {
        XY { x, y }
    }

    /// Swaps the x and y values.
    pub fn swap(self) -> Self {
        XY::new(self.y, self.x)
    }

    /// Returns `f(self.x, self.y)`
    pub fn fold<U, F>(self, f: F) -> U
    where
        F: FnOnce(T, T) -> U,
    {
        f(self.x, self.y)
    }

    /// Creates a new `XY` by applying `f` to `x` and `y`.
    pub fn map<U, F>(self, f: F) -> XY<U>
    where
        F: Fn(T) -> U,
    {
        XY::new(f(self.x), f(self.y))
    }

    /// Applies `f` on axis where `condition` is true.
    ///
    /// Carries over `self` otherwise.
    pub fn map_if<F>(self, condition: XY<bool>, f: F) -> Self
    where
        F: Fn(T) -> T,
    {
        self.zip_map(condition, |v, c| if c { f(v) } else { v })
    }

    /// Applies `f` on axis where `condition` is true.
    ///
    /// Returns `None` otherwise.
    pub fn run_if<F, U>(self, condition: XY<bool>, f: F) -> XY<Option<U>>
    where
        F: Fn(T) -> U,
    {
        self.zip_map(condition, |v, c| if c { Some(f(v)) } else { None })
    }

    /// Creates a new `XY` by applying `f` to `x`, and carrying `y` over.
    pub fn map_x<F>(self, f: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        XY::new(f(self.x), self.y)
    }

    /// Creates a new `XY` by applying `f` to `y`, and carrying `x` over.
    pub fn map_y<F>(self, f: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        XY::new(self.x, f(self.y))
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

    /// Returns a mutable reference to the value on the given axis.
    pub fn get_mut(&mut self, o: Orientation) -> &mut T {
        match o {
            Orientation::Horizontal => &mut self.x,
            Orientation::Vertical => &mut self.y,
        }
    }

    /// Returns a new `XY` of tuples made by zipping `self` and `other`.
    pub fn zip<U>(self, other: XY<U>) -> XY<(T, U)> {
        XY::new((self.x, other.x), (self.y, other.y))
    }

    /// Returns a new `XY` of tuples made by zipping `self`, `a` and `b`.
    pub fn zip3<U, V>(self, a: XY<U>, b: XY<V>) -> XY<(T, U, V)> {
        XY::new((self.x, a.x, b.x), (self.y, a.y, b.y))
    }

    /// Returns a new `XY` of tuples made by zipping `self`, `a`, `b` and `c`.
    pub fn zip4<U, V, W>(
        self, a: XY<U>, b: XY<V>, c: XY<W>,
    ) -> XY<(T, U, V, W)> {
        XY::new((self.x, a.x, b.x, c.x), (self.y, a.y, b.y, c.y))
    }

    /// Returns a new `XY` of tuples made by zipping `self`, `a`, `b`, `c` and `d`.
    pub fn zip5<U, V, W, Z>(
        self, a: XY<U>, b: XY<V>, c: XY<W>, d: XY<Z>,
    ) -> XY<(T, U, V, W, Z)> {
        XY::new((self.x, a.x, b.x, c.x, d.x), (self.y, a.y, b.y, c.y, d.y))
    }

    /// Returns a new `XY` by calling `f` on `self` and `other` for each axis.
    pub fn zip_map<U, V, F>(self, other: XY<U>, f: F) -> XY<V>
    where
        F: Fn(T, U) -> V,
    {
        XY::new(f(self.x, other.x), f(self.y, other.y))
    }

    /// For each axis, keep the element from `self` if `keep` is `true`.
    pub fn keep(self, keep: XY<bool>) -> XY<Option<T>> {
        keep.select(self)
    }
}

impl<T: Clone> XY<T> {
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
        self.zip_map(other, Option::unwrap_or)
    }
}

impl XY<bool> {
    /// Returns `true` if any of `x` or `y` is `true`.
    pub fn any(self) -> bool {
        use std::ops::BitOr;
        self.fold(BitOr::bitor)
    }

    /// Returns `true` if both `x` and `y` are `true`.
    pub fn both(self) -> bool {
        use std::ops::BitAnd;
        self.fold(BitAnd::bitand)
    }

    /// For each axis, keeps elements from `other` if `self` is `true`.
    pub fn select<T>(self, other: XY<T>) -> XY<Option<T>> {
        self.zip_map(other, |keep, o| if keep { Some(o) } else { None })
    }

    /// For each axis, selects `if_true` if `self` is true, else `if_false`.
    pub fn select_or<T>(self, if_true: XY<T>, if_false: XY<T>) -> XY<T> {
        self.select(if_true).unwrap_or(if_false)
    }

    /// Returns a term-by-term AND operation.
    pub fn and(self, other: Self) -> Self {
        self.zip_map(other, |s, o| s && o)
    }

    /// Returns a term-by-term OR operation.
    pub fn or(self, other: Self) -> Self {
        self.zip_map(other, |s, o| s || o)
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

impl<T, U> From<(XY<T>, XY<U>)> for XY<(T, U)> {
    fn from((t, u): (XY<T>, XY<U>)) -> Self {
        t.zip(u)
    }
}
