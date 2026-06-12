use enum_map::Enum;

use std::str::FromStr;

/// Specifies how some borders should be drawn.
///
/// Borders are used around Dialogs, select popups, and panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum BorderStyle {
    /// Simple borders.
    Simple,
    /// Outset borders with a simple 3d effect.
    Outset,
    /// No borders.
    None,
}

impl BorderStyle {
    /// Returns an iterator on all possible border styles.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }
}

impl FromStr for BorderStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "simple" => Ok(BorderStyle::Simple),
            "outset" => Ok(BorderStyle::Outset),
            "none" => Ok(BorderStyle::None),
            _ => Err(()),
        }
    }
}
