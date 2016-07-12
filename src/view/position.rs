use std::cmp::min;
use vec::Vec2;

/// Location of the view on screen
#[derive(PartialEq,Debug,Clone)]
pub struct Position {
    /// Horizontal offset
    pub x: Offset,
    /// Vertical offset
    pub y: Offset,
}

impl Position {
    /// Creates a new position with the given offsets.
    pub fn new(x: Offset, y: Offset) -> Self {
        Position { x: x, y: y }
    }

    /// Returns a position centered on both axis.
    pub fn center() -> Self {
        Position::new(Offset::Center, Offset::Center)
    }

    /// Returns a position absolute on both axis.
    pub fn absolute<T: Into<Vec2>>(offset: T) -> Self {
        let offset = offset.into();
        Position::new(Offset::Absolute(offset.x), Offset::Absolute(offset.y))
    }

    /// Returns a position relative to the parent on both axis.
    pub fn parent<T: Into<Vec2>>(offset: T) -> Self {
        let offset = offset.into();
        Position::new(Offset::Parent(offset.x), Offset::Parent(offset.y))
    }

    /// Computes the offset required to draw a view.
    ///
    /// When drawing a view with `size` in a container with `available`,
    /// and a parent with the absolute coordinates `parent`, drawing the
    /// child with its top-left corner at the returned coordinates will
    /// position him appropriately.
    pub fn compute_offset(&self, size: Vec2, available: Vec2, parent: Vec2)
                          -> Vec2 {
        Vec2::new(self.x.compute_offset(size.x, available.x, parent.x),
                  self.y.compute_offset(size.y, available.y, parent.y))
    }
}

/// Single-dimensional offset policy.
#[derive(PartialEq,Debug,Clone)]
pub enum Offset {
    /// In the center of the screen
    Center,
    /// Place top-left corner at the given absolute coordinates
    Absolute(usize),

    /// Offset from the previous layer's top-left corner.
    ///
    /// If this is the first layer, behaves like `Absolute`.
    Parent(usize), // TODO: use a signed vec for negative offset?
}

impl Offset {
    /// Computes a single-dimension offset requred to draw a view.
    pub fn compute_offset(&self, size: usize, available: usize, parent: usize)
                          -> usize {
        match *self {
            Offset::Center => (available - size) / 2,
            Offset::Absolute(offset) => min(offset, available - size),
            Offset::Parent(offset) => min(parent + offset, available - size),
        }
    }
}
