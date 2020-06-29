use crate::direction::Orientation;
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

    /// Iterate over x, then y.
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// assert_eq!(xy.swap(), XY::new(2, 1));
    /// ```
    pub fn swap(self) -> Self {
        XY::new(self.y, self.x)
    }

    /// Returns `f(self.x, self.y)`
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    ///
    /// assert_eq!(xy.fold(std::ops::Add::add), 3);
    /// assert_eq!(xy.fold(std::ops::Mul::mul), 2);
    /// assert_eq!(xy.fold(|x, y| x < y), true);
    /// ```
    pub fn fold<U, F>(self, f: F) -> U
    where
        F: FnOnce(T, T) -> U,
    {
        f(self.x, self.y)
    }

    /// Creates a new `XY` by applying `f` to `x` and `y`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    ///
    /// assert_eq!(xy.map(|v| v * 2), XY::new(2, 4));
    /// assert_eq!(xy.map(|v| v > 1), XY::new(false, true));
    /// ```
    pub fn map<U, F>(self, f: F) -> XY<U>
    where
        F: Fn(T) -> U,
    {
        XY::new(f(self.x), f(self.y))
    }

    /// Applies `f` on axis where `condition` is true.
    ///
    /// Carries over `self` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// let cond = XY::new(true, false);
    ///
    /// assert_eq!(xy.map_if(cond, |v| v * 3), XY::new(3, 2));
    ///
    /// ```
    pub fn map_if<F>(self, condition: XY<bool>, f: F) -> Self
    where
        F: Fn(T) -> T,
    {
        self.zip_map(condition, |v, c| if c { f(v) } else { v })
    }

    /// Applies `f` on axis where `condition` is true.
    ///
    /// Returns `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// let cond = XY::new(true, false);
    ///
    /// assert_eq!(xy.run_if(cond, |v| v * 3), XY::new(Some(3), None));
    /// ```
    pub fn run_if<F, U>(self, condition: XY<bool>, f: F) -> XY<Option<U>>
    where
        F: Fn(T) -> U,
    {
        self.zip_map(condition, |v, c| if c { Some(f(v)) } else { None })
    }

    /// Creates a new `XY` by applying `f` to `x`, and carrying `y` over.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// assert_eq!(xy.map_x(|x| x * 10), XY::new(10, 2));
    /// ```
    pub fn map_x<F>(self, f: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        XY::new(f(self.x), self.y)
    }

    /// Creates a new `XY` by applying `f` to `y`, and carrying `x` over.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// assert_eq!(xy.map_y(|y| y * 10), XY::new(1, 20));
    /// ```
    pub fn map_y<F>(self, f: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        XY::new(self.x, f(self.y))
    }

    /// Destructure self into a pair.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// let (x, y) = xy.pair();
    /// assert_eq!((x, y), (1, 2));
    /// ```
    pub fn pair(self) -> (T, T) {
        (self.x, self.y)
    }

    /// Return a `XY` with references to this one's values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// fn longer(xy: &XY<String>, l: usize) -> XY<bool> {
    ///     // `XY::map` takes ownership
    ///     // So we need to get a XY<&String> from a &XY<String>
    ///     let by_ref: XY<&String> = xy.as_ref();
    ///     by_ref.map(|s| s.len() > l)
    /// }
    ///
    /// let xy = XY::new(String::from("a"), String::from("bbb"));
    ///
    /// assert_eq!(longer(&xy, 2), XY::new(false, true));
    /// ```
    pub fn as_ref(&self) -> XY<&T> {
        XY::new(&self.x, &self.y)
    }

    /// Creates an iterator that returns references to `x`, then `y`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// let vec: Vec<bool> = xy.iter().map(|&i| i > 1).collect();
    /// assert_eq!(vec, vec![false, true]);
    /// ```
    pub fn iter(&self) -> iter::Chain<iter::Once<&T>, iter::Once<&T>> {
        iter::once(&self.x).chain(iter::once(&self.y))
    }

    /// Returns a reference to the value on the given axis.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// # use cursive_core::direction::Orientation;
    /// let xy = XY::new(1, 2);
    /// assert_eq!(xy.get(Orientation::Horizontal), &1);
    /// assert_eq!(xy.get(Orientation::Vertical), &2);
    /// ```
    pub fn get(&self, o: Orientation) -> &T {
        match o {
            Orientation::Horizontal => &self.x,
            Orientation::Vertical => &self.y,
        }
    }

    /// Returns a mutable reference to the value on the given axis.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// # use cursive_core::direction::Orientation;
    /// let mut xy = XY::new(1, 2);
    /// *xy.get_mut(Orientation::Horizontal) = 10;
    ///
    /// assert_eq!(xy, XY::new(10, 2));
    /// ```
    pub fn get_mut(&mut self, o: Orientation) -> &mut T {
        match o {
            Orientation::Horizontal => &mut self.x,
            Orientation::Vertical => &mut self.y,
        }
    }

    /// Returns a new `XY` of tuples made by zipping `self` and `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let a = XY::new(1, 2);
    /// let b = XY::new(true, false);
    /// assert_eq!(a.zip(b), XY::new((1, true), (2, false)));
    /// ```
    pub fn zip<U>(self, other: XY<U>) -> XY<(T, U)> {
        XY::new((self.x, other.x), (self.y, other.y))
    }

    /// Returns a new `XY` of tuples made by zipping `self`, `a` and `b`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let a = XY::new(1, 2);
    /// let b = XY::new(true, false);
    /// let c = XY::new("x", "y");
    /// assert_eq!(a.zip3(b, c), XY::new((1, true, "x"), (2, false, "y")));
    /// ```
    pub fn zip3<U, V>(self, a: XY<U>, b: XY<V>) -> XY<(T, U, V)> {
        XY::new((self.x, a.x, b.x), (self.y, a.y, b.y))
    }

    /// Returns a new `XY` of tuples made by zipping `self`, `a`, `b` and `c`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let a = XY::new(1, 2);
    /// let b = XY::new(true, false);
    /// let c = XY::new("x", "y");
    /// let d = XY::new(vec![1], vec![2, 3, 4]);
    /// assert_eq!(
    ///     XY::zip4(a, b, c, d),
    ///     XY::new((1, true, "x", vec![1]), (2, false, "y", vec![2, 3, 4]))
    /// );
    /// ```
    pub fn zip4<U, V, W>(
        self,
        a: XY<U>,
        b: XY<V>,
        c: XY<W>,
    ) -> XY<(T, U, V, W)> {
        XY::new((self.x, a.x, b.x, c.x), (self.y, a.y, b.y, c.y))
    }

    /// Returns a new `XY` of tuples made by zipping `self`, `a`, `b`, `c` and `d`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let a = XY::new(1, 2);
    /// let b = XY::new(true, false);
    /// let c = XY::new("x", "y");
    /// let d = XY::new(vec![1], vec![2, 3, 4]);
    /// let e = XY::new('a', 'b');
    ///
    /// let xy: XY<Option<char>> = XY::zip5(a, b, c, d, e)
    ///     .map(|(a, b, c, d, e)| {
    ///         if b && d.contains(&a) {
    ///             Some(e)
    ///         } else {
    ///             c.chars().next()
    ///         }
    ///     });
    /// assert_eq!(xy, XY::new(Some('a'), Some('y')));
    /// ```
    pub fn zip5<U, V, W, Z>(
        self,
        a: XY<U>,
        b: XY<V>,
        c: XY<W>,
        d: XY<Z>,
    ) -> XY<(T, U, V, W, Z)> {
        XY::new((self.x, a.x, b.x, c.x, d.x), (self.y, a.y, b.y, c.y, d.y))
    }

    /// Returns a new `XY` by calling `f` on `self` and `other` for each axis.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let a = XY::new((1, 10), (2, 20));
    /// let b = XY::new(true, false);
    /// let xy = a.zip_map(b, |(a1, a2), b| if b { a1 } else { a2 });
    /// assert_eq!(xy, XY::new(1, 20));
    /// ```
    pub fn zip_map<U, V, F>(self, other: XY<U>, f: F) -> XY<V>
    where
        F: Fn(T, U) -> V,
    {
        XY::new(f(self.x, other.x), f(self.y, other.y))
    }

    /// For each axis, keep the element from `self` if `keep` is `true`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(1, 2);
    /// let cond = XY::new(true, false);
    ///
    /// assert_eq!(xy.keep(cond), XY::new(Some(1), None));
    /// ```
    pub fn keep(self, keep: XY<bool>) -> XY<Option<T>> {
        keep.select(self)
    }
}

impl<T: Clone> XY<T> {
    /// Returns a new `XY` with the axis `o` set to `value`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// # use cursive_core::direction::Orientation;
    /// let xy = XY::new(1, 2).with_axis(Orientation::Horizontal, 42);
    ///
    /// assert_eq!(xy, XY::new(42, 2));
    /// ```
    pub fn with_axis(&self, o: Orientation, value: T) -> Self {
        let mut new = self.clone();
        *o.get_ref(&mut new) = value;
        new
    }

    /// Returns a new `XY` with the axis `o` set to the value from `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// # use cursive_core::direction::Orientation;
    /// let other = XY::new(3, 4);
    /// let xy = XY::new(1, 2).with_axis_from(Orientation::Horizontal, &other);
    ///
    /// assert_eq!(xy, XY::new(3, 2));
    /// ```
    pub fn with_axis_from(&self, o: Orientation, other: &Self) -> Self {
        let mut new = self.clone();
        new.set_axis_from(o, other);
        new
    }

    /// Sets the axis `o` on `self` to the value from `other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// # use cursive_core::direction::Orientation;
    /// let mut xy = XY::new(1, 2);
    /// let other = XY::new(3, 4);
    /// xy.set_axis_from(Orientation::Horizontal, &other);
    ///
    /// assert_eq!(xy, XY::new(3, 2));
    /// ```
    pub fn set_axis_from(&mut self, o: Orientation, other: &Self) {
        *o.get_ref(self) = o.get(other);
    }

    /// Creates a `XY` with both `x` and `y` set to `value`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::both_from(42);
    ///
    /// assert_eq!(xy, XY::new(42, 42));
    /// ```
    pub fn both_from(value: T) -> Self {
        let x = value.clone();
        let y = value;
        XY::new(x, y)
    }
}

impl<T> XY<Option<T>> {
    /// Returns a new `XY` by calling `unwrap_or` on each axis.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let xy = XY::new(Some(1), None);
    /// assert_eq!(xy.unwrap_or(XY::new(10, 20)), XY::new(1, 20));
    /// ```
    pub fn unwrap_or(self, other: XY<T>) -> XY<T> {
        self.zip_map(other, Option::unwrap_or)
    }

    /// Returns a new `XY` if both components are present in `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// assert_eq!(XY::new(Some(1), None).both(), None);
    /// assert_eq!(XY::new(Some(1), Some(2)).both(), Some(XY::new(1, 2)));
    /// ```
    pub fn both(self) -> Option<XY<T>> {
        match self {
            XY {
                x: Some(x),
                y: Some(y),
            } => Some(XY::new(x, y)),
            _ => None,
        }
    }
}

impl XY<bool> {
    // Could also be called "either"
    /// Returns `true` if any of `x` or `y` is `true`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// assert_eq!(XY::new(true, false).any(), true);
    /// assert_eq!(XY::new(false, false).any(), false);
    /// assert_eq!(XY::new('a', 'b').map(|c| c == 'a').any(), true);
    /// ```
    pub fn any(self) -> bool {
        use std::ops::BitOr;
        self.fold(BitOr::bitor)
    }

    // Could also be called "all"
    /// Returns `true` if both `x` and `y` are `true`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// assert_eq!(XY::new(true, false).both(), false);
    /// assert_eq!(XY::new(true, true).both(), true);
    /// assert_eq!(XY::new("abc", "de").map(|s| s.len() > 2).both(), false);
    /// ```
    pub fn both(self) -> bool {
        use std::ops::BitAnd;
        self.fold(BitAnd::bitand)
    }

    /// For each axis, keeps elements from `other` if `self` is `true`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let choice = XY::new(true, false);
    /// let values = XY::new(1, 2);
    /// let selection = choice.select(values);
    ///
    /// assert_eq!(selection, XY::new(Some(1), None));
    /// ```
    pub fn select<T>(self, other: XY<T>) -> XY<Option<T>> {
        self.zip_map(other, |keep, o| if keep { Some(o) } else { None })
    }

    /// For each axis, selects `if_true` if `self` is true, else `if_false`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let choice = XY::new(true, false);
    /// let values = XY::new(1, 2);
    /// let fallback = XY::new(3, 4);
    /// let selection = choice.select_or(values, fallback);
    ///
    /// assert_eq!(selection, XY::new(1, 4));
    /// ```
    pub fn select_or<T>(self, if_true: XY<T>, if_false: XY<T>) -> XY<T> {
        self.select(if_true).unwrap_or(if_false)
    }

    /// Returns a term-by-term AND operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let a = XY::new(true, false);
    /// let b = XY::new(true, true);
    /// assert_eq!(a.and(b), XY::new(true, false));
    /// ```
    pub fn and(self, other: Self) -> Self {
        self.zip_map(other, |s, o| s && o)
    }

    /// Returns a term-by-term OR operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::XY;
    /// let a = XY::new(true, false);
    /// let b = XY::new(true, true);
    /// assert_eq!(a.or(b), XY::new(true, true));
    /// ```
    pub fn or(self, other: Self) -> Self {
        self.zip_map(other, |s, o| s || o)
    }
}

impl<T> From<(T, T)> for XY<T> {
    /// A pair is assumed to be (x, y)
    fn from((x, y): (T, T)) -> Self {
        XY::new(x, y)
    }
}

impl<T, U> From<(XY<T>, XY<U>)> for XY<(T, U)> {
    /// Easily zip a pair of XY into a XY of pair
    fn from((t, u): (XY<T>, XY<U>)) -> Self {
        t.zip(u)
    }
}
