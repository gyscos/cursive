//! Direction-related structures.
//!
//! This module defines two main concepts: [`Orientation`] and [`Direction`].
//!
//! [`Orientation`]: enum.Orientation.html
//! [`Direction`]: enum.Direction.html
//!
//! ### Orientation
//!
//! `Orientation` is a simple enum that can take two values:
//! `Horizontal` or `Vertical`.
//!
//! ### Direction
//!
//! `Direction` is a bit more complex, and can be of two kinds:
//!
//! * Absolute direction: left, up, right, or down
//! * Relative direction: front or back. Its actual direction depends on the
//!   orientation.
//!
//!   Usually, "front" refers to the "forward" direction, or the "next"
//!   element. For example, for a vertical `LinearLayout`, "front" would refer
//!   to the "down" direction.
//!
//!   This is mostly relevant when referring to change of focus. Hitting the
//!   `Tab` key would usually cycle focus in the "front" direction, while
//!   using the arrow keys would use absolute directions instead.

use crate::Vec2;
use crate::XY;

/// Describes a vertical or horizontal orientation for a view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Horizontal orientation
    Horizontal,
    /// Vertical orientation
    Vertical,
}

impl std::str::FromStr for Orientation {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Vertical" | "vertical" => Self::Vertical,
            "Horizontal" | "horizontal" => Self::Horizontal,
            _ => return Err(()),
        })
    }
}

impl Orientation {
    /// Returns a `XY(Horizontal, Vertical)`.
    pub const fn pair() -> XY<Orientation> {
        XY::new(Orientation::Horizontal, Orientation::Vertical)
    }

    /// Returns the component of `v` corresponding to this orientation.
    ///
    /// (`Horizontal` will return the x value,
    /// and `Vertical` will return the y value.)
    pub fn get<T: Clone>(self, v: &XY<T>) -> T {
        v.get(self).clone()
    }

    /// Returns the other orientation.
    #[must_use]
    pub const fn swap(self) -> Self {
        match self {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
        }
    }

    // /// Returns a reference to the component of the given vector
    // /// corresponding to this orientation.
    // pub const fn get_ref<T>(self, v: &XY<T>) -> &T {
    //     match self {
    //         Orientation::Horizontal => &v.x,
    //         Orientation::Vertical => &v.y,
    //     }
    // }

    /// Returns a mutable reference to the component of the given vector
    /// corresponding to this orientation.
    ///
    /// # Examples
    /// ```rust
    /// # use cursive_core::XY;
    /// # use cursive_core::direction::Orientation;
    /// let o = Orientation::Horizontal;
    /// let mut xy = XY::new(1, 2);
    /// *o.get_mut(&mut xy) = 42;
    ///
    /// assert_eq!(xy, XY::new(42, 2));
    /// ```
    pub fn get_mut<T>(self, v: &mut XY<T>) -> &mut T {
        match self {
            Orientation::Horizontal => &mut v.x,
            Orientation::Vertical => &mut v.y,
        }
    }

    /// Same as [`Self::get_mut()`].
    #[deprecated]
    pub fn get_ref<T>(self, v: &mut XY<T>) -> &mut T {
        self.get_mut(v)
    }

    /// Takes an iterator on sizes, and stack them in the current orientation,
    /// returning the size of the required bounding box.
    ///
    /// * For an horizontal view, returns `(Sum(x), Max(y))`.
    /// * For a vertical view, returns `(Max(x), Sum(y))`.
    pub fn stack<T: Iterator<Item = Vec2>>(self, iter: T) -> Vec2 {
        match self {
            Orientation::Horizontal => iter.fold(Vec2::zero(), |a, b| a.stack_horizontal(&b)),
            Orientation::Vertical => iter.fold(Vec2::zero(), |a, b| a.stack_vertical(&b)),
        }
    }

    /// Creates a new `Vec2` with `main_axis` in `self`'s axis, and
    /// `second_axis` for the other axis.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::direction::Orientation;
    /// # use cursive_core::Vec2;
    /// let o = Orientation::Horizontal;
    /// let vec = o.make_vec(1, 2);
    ///
    /// assert_eq!(vec, Vec2::new(1, 2));
    ///
    /// let o = Orientation::Vertical;
    /// let vec = o.make_vec(1, 2);
    ///
    /// assert_eq!(vec, Vec2::new(2, 1));
    /// ```
    pub const fn make_vec(self, main_axis: usize, second_axis: usize) -> Vec2 {
        Vec2::from_major_minor(self, main_axis, second_axis)
    }
}

/// Represents a direction, either absolute or orientation-dependent.
///
/// * Absolute directions are Up, Down, Left, and Right.
/// * Relative directions are Front and Back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// An absolute direction.
    Abs(Absolute),
    /// A direction relative to the current orientation.
    Rel(Relative),
}

impl std::str::FromStr for Direction {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse()
            .map(Direction::Abs)
            .or_else(|_| s.parse().map(Direction::Rel))
    }
}

impl Direction {
    /// Returns the relative direction for the given orientation.
    ///
    /// Some combination have no corresponding relative position. For example,
    /// `Direction::Abs(Up)` means nothing for `Orientation::Horizontal`.
    pub const fn relative(self, orientation: Orientation) -> Option<Relative> {
        match self {
            Direction::Abs(abs) => abs.relative(orientation),
            Direction::Rel(rel) => Some(rel),
        }
    }

    /// Returns the absolute direction in the given `orientation`.
    pub const fn absolute(self, orientation: Orientation) -> Absolute {
        match self {
            Direction::Abs(abs) => abs,
            Direction::Rel(rel) => rel.absolute(orientation),
        }
    }

    /// Returns the direction opposite `self`.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Direction::Abs(abs) => Direction::Abs(abs.opposite()),
            Direction::Rel(rel) => Direction::Rel(rel.swap()),
        }
    }

    /// Shortcut to create `Direction::Rel(Relative::Back)`
    pub const fn back() -> Self {
        Direction::Rel(Relative::Back)
    }

    /// Shortcut to create `Direction::Rel(Relative::Front)`
    pub const fn front() -> Self {
        Direction::Rel(Relative::Front)
    }

    /// Shortcut to create `Direction::Abs(Absolute::Left)`
    pub const fn left() -> Self {
        Direction::Abs(Absolute::Left)
    }

    /// Shortcut to create `Direction::Abs(Absolute::Right)`
    pub const fn right() -> Self {
        Direction::Abs(Absolute::Right)
    }

    /// Shortcut to create `Direction::Abs(Absolute::Up)`
    pub const fn up() -> Self {
        Direction::Abs(Absolute::Up)
    }

    /// Shortcut to create `Direction::Abs(Absolute::Down)`
    pub const fn down() -> Self {
        Direction::Abs(Absolute::Down)
    }

    /// Shortcut to create `Direction::Abs(Absolute::None)`
    pub const fn none() -> Self {
        Direction::Abs(Absolute::None)
    }
}

/// Direction relative to an orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relative {
    // TODO: handle right-to-left? (Arabic, ...)
    /// Front relative direction.
    ///
    /// * Horizontally, this means `Left`
    /// * Vertically, this means `Up`
    Front,

    /// Back relative direction.
    ///
    /// * Horizontally, this means `Right`
    /// * Vertically, this means `Down`.
    Back,
}

impl std::str::FromStr for Relative {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Front" | "front" => Self::Front,
            "Back" | "back" => Self::Back,
            _ => return Err(()),
        })
    }
}

impl Relative {
    /// Returns the absolute direction in the given `orientation`.
    pub const fn absolute(self, orientation: Orientation) -> Absolute {
        match (orientation, self) {
            (Orientation::Horizontal, Relative::Front) => Absolute::Left,
            (Orientation::Horizontal, Relative::Back) => Absolute::Right,
            (Orientation::Vertical, Relative::Front) => Absolute::Up,
            (Orientation::Vertical, Relative::Back) => Absolute::Down,
        }
    }

    /// Picks one of the two values in a tuple.
    ///
    /// First one is `self` is `Front`, second one if `self` is `Back`.
    pub fn pick<T>(self, (front, back): (T, T)) -> T {
        match self {
            Relative::Front => front,
            Relative::Back => back,
        }
    }

    /// Returns the other relative direction.
    #[must_use]
    pub const fn swap(self) -> Self {
        match self {
            Relative::Front => Relative::Back,
            Relative::Back => Relative::Front,
        }
    }

    /// Returns the position of `a` relative to `b`.
    ///
    /// If `a < b`, it would be `Front`.
    /// If `a > b`, it would be `Back`.
    /// If `a == b`, returns `None`.
    pub const fn a_to_b(a: usize, b: usize) -> Option<Self> {
        // TODO: use std::cmp::Ordering once const trait are a thing.
        match (a < b, a == b) {
            (_, true) => None,
            (true, false) => Some(Relative::Front),
            (false, false) => Some(Relative::Back),
        }
    }
}

/// Absolute direction (up, down, left, right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Absolute {
    /// Left
    Left,
    /// Up
    Up,
    /// Right
    Right,
    /// Down
    Down,

    /// No real direction.
    ///
    /// Used when the "direction" is across layers for instance.
    None,
}

impl std::str::FromStr for Absolute {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Left" | "left" => Self::Left,
            "Up" | "up" => Self::Up,
            "Right" | "right" => Self::Right,
            "Down" | "down" => Self::Down,
            "None" | "none" => Self::None,
            _ => return Err(()),
        })
    }
}

impl Absolute {
    /// Returns the relative direction for the given orientation.
    ///
    /// Returns `None` when the direction does not apply to the given
    /// orientation (ex: `Left` and `Vertical`).
    pub const fn relative(self, orientation: Orientation) -> Option<Relative> {
        match (orientation, self) {
            (Orientation::Horizontal, Absolute::Left) | (Orientation::Vertical, Absolute::Up) => {
                Some(Relative::Front)
            }
            (Orientation::Horizontal, Absolute::Right)
            | (Orientation::Vertical, Absolute::Down) => Some(Relative::Back),
            _ => None,
        }
    }

    /// Returns the direction opposite `self`.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Absolute::Left => Absolute::Right,
            Absolute::Right => Absolute::Left,
            Absolute::Up => Absolute::Down,
            Absolute::Down => Absolute::Up,
            Absolute::None => Absolute::None,
        }
    }

    /// Splits this absolute direction into an orientation and relative direction.
    ///
    /// For example, `Right` will give `(Horizontal, Back)`.
    pub const fn split(self) -> (Orientation, Relative) {
        match self {
            Absolute::Left => (Orientation::Horizontal, Relative::Front),
            Absolute::Right => (Orientation::Horizontal, Relative::Back),
            Absolute::Up => (Orientation::Vertical, Relative::Front),
            Absolute::Down => (Orientation::Vertical, Relative::Back),
            // TODO: Remove `Absolute::None`
            Absolute::None => panic!("None direction not supported here"),
        }
    }
}
