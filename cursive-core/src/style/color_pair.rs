use super::Color;

/// Combines a front and back color.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
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
    #[must_use]
    pub const fn invert(self) -> Self {
        ColorPair {
            front: self.back,
            back: self.front,
        }
    }

    /// Return a color with `TerminalDefault` as front and back.
    pub const fn terminal_default() -> Self {
        Self {
            front: Color::TerminalDefault,
            back: Color::TerminalDefault,
        }
    }

    /// Creates a new color pair from color IDs.
    #[must_use]
    pub const fn from_256colors(front: u8, back: u8) -> Self {
        Self {
            front: Color::from_256colors(front),
            back: Color::from_256colors(back),
        }
    }
}
