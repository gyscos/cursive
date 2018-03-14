//! Internal module for some implementation details.
//!
//! You probable are not interested in anything here. These elements are
//! required to be public, but you should never need to use them directly.

use view::{View, AnyView};

/// A trait for elements that can be converted to `AnyView`.
///
/// You should never need to implement this yourself; it is automatically
/// implemented for any `T: View`.
///
/// Just ignore this trait entirely.
pub trait ToAny {
    /// Converts a boxed `Self` to a boxed `AnyView`.
    fn to_any(self: Box<Self>) -> Box<AnyView>;
}

impl <T: View> ToAny for T {
    fn to_any(self: Box<Self>) -> Box<AnyView> {
        self
    }
}
