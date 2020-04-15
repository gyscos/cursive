use std::ops::Deref;

/// Specifies how some borders should be drawn.
///
/// Borders are used around Dialogs, select popups, and panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorderStyle {
    /// Simple borders.
    Simple,
    /// Outset borders with a simple 3d effect.
    Outset,
    /// No borders.
    None,
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
