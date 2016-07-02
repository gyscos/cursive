use ::vec::{ToVec2, Vec2};

/// Location of the view on screen
pub struct Position {
    pub x: Offset,
    pub y: Offset,
}

impl Position {
    pub fn new(x: Offset, y: Offset) -> Self {
        Position {
            x: x,
            y: y,
        }
    }

    pub fn center() -> Self {
        Position::new(Offset::Center, Offset::Center)
    }

    pub fn absolute<T: ToVec2>(offset: T) -> Self {
        let offset = offset.to_vec2();
        Position::new(Offset::Absolute(offset.x), Offset::Absolute(offset.y))
    }

    pub fn parent<T: ToVec2>(offset: T) -> Self {
        let offset = offset.to_vec2();
        Position::new(Offset::Parent(offset.x), Offset::Parent(offset.y))
    }

    pub fn compute_offset(&self, size: Vec2, available: Vec2, parent: Vec2) -> Vec2 {
        Vec2::new(self.x.compute_offset(size.x, available.x, parent.x),
                  self.y.compute_offset(size.y, available.y, parent.y))
    }
}

pub enum Offset {
    /// In the center of the screen
    Center,
    /// Place top-left corner at the given absolute coordinates
    Absolute(usize),

    /// Place top-left corner at the given offset from the previous layer's top-left corner.
    ///
    /// If this is the first layer, behaves like `Absolute`.
    Parent(usize), // TODO: use a signed vec for negative offset?
}

impl Offset {
    pub fn compute_offset(&self, size: usize, available: usize, parent: usize) -> usize {
        match *self {
            Offset::Center => (available - size) / 2,
            Offset::Absolute(offset) => offset,
            Offset::Parent(offset) => parent + offset,
        }
    }
}
