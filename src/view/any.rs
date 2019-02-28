use std::any::Any;
use crate::view::View;

/// A view that can be downcasted to its concrete type.
///
/// This trait is automatically implemented for any `T: View`.
pub trait AnyView {
    /// Downcast self to a `Any`.
    fn as_any(&self) -> &Any;

    /// Downcast self to a mutable `Any`.
    fn as_any_mut(&mut self) -> &mut Any;

    /// Returns a boxed any from a boxed self.
    ///
    /// Can be used before `Box::downcast()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::views::TextView;
    /// # use cursive::view::View;
    /// # fn main() {
    /// let boxed: Box<View> = Box::new(TextView::new("text"));
    /// let text: Box<TextView> = boxed.as_boxed_any().downcast().unwrap();
    /// # }
    /// ```
    fn as_boxed_any(self: Box<Self>) -> Box<Any>;
}

impl<T: View> AnyView for T {
    /// Downcast self to a `Any`.
    fn as_any(&self) -> &Any {
        self
    }

    /// Downcast self to a mutable `Any`.
    fn as_any_mut(&mut self) -> &mut Any {
        self
    }

    fn as_boxed_any(self: Box<Self>) -> Box<Any> {
        self
    }
}
