use std::cmp::min;
use vec::Vec2;

/// Location of the view on screen
#[derive(PartialEq,Debug,Clone)]
pub struct Position {
    pub x: Offset,
    pub y: Offset,
}

impl Position {
    pub fn new(x: Offset, y: Offset) -> Self {
        Position { x: x, y: y }
    }

    pub fn center() -> Self {
        Position::new(Offset::Center, Offset::Center)
    }

    pub fn absolute<T: Into<Vec2>>(offset: T) -> Self {
        let offset = offset.into();
        Position::new(Offset::Absolute(offset.x), Offset::Absolute(offset.y))
    }

    pub fn parent<T: Into<Vec2>>(offset: T) -> Self {
        let offset = offset.into();
        Position::new(Offset::Parent(offset.x), Offset::Parent(offset.y))
    }

    pub fn compute_offset(&self, size: Vec2, available: Vec2, parent: Vec2)
                          -> Vec2 {
        Vec2::new(self.x.compute_offset(size.x, available.x, parent.x),
                  self.y.compute_offset(size.y, available.y, parent.y))
    }
}

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
    pub fn compute_offset(&self, size: usize, available: usize, parent: usize)
                          -> usize {
        match *self {
            Offset::Center => (available - size) / 2,
            Offset::Absolute(offset) => min(offset, available - size),
            Offset::Parent(offset) => min(parent + offset, available - size),
        }
    }
}
