use view::View;

/// Represents a type that can be made into a `Box<View>`.
pub trait IntoBoxedView {
    /// Returns a `Box<View>`.
    fn as_boxed_view(self) -> Box<View>;
}

impl<T> IntoBoxedView for T
where
    T: View,
{
    fn as_boxed_view(self) -> Box<View> {
        Box::new(self)
    }
}

impl IntoBoxedView for Box<View> {
    fn as_boxed_view(self) -> Box<View> {
        self
    }
}
