use super::Color;

/// Combines a front and back color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ColorPair {
    /// Color used for the foreground.
    pub front: Color,

    /// Color used for the background.
    pub back: Color,
}

impl ColorPair {
    /// Return an inverted color pair.
    ///
    /// With swapped front and back color.
    pub fn invert(self) -> Self {
        ColorPair {
            front: self.back,
            back: self.front,
        }
    }

    /// Creates a new color pair from color IDs.
    pub fn from_256colors(front: u8, back: u8) -> Self {
        Self {
            front: Color::from_256colors(front),
            back: Color::from_256colors(back),
        }
    }
}
