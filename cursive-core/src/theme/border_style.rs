use enum_map::Enum;

use std::ops::Deref;

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

impl<S: Deref<Target = String>> From<S> for BorderStyle {
    fn from(s: S) -> Self {
        if &*s == "simple" {
            BorderStyle::Simple
        } else if &*s == "outset" {
            BorderStyle::Outset
        } else {
            BorderStyle::None
        }
    }
}
