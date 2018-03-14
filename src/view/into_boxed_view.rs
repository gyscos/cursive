use view::{AnyView, View};

/// Represents a type that can be made into a `Box<AnyView>`.
pub trait IntoBoxedView {
    /// Returns a `Box<AnyView>`.
    fn as_boxed_view(self) -> Box<AnyView>;
}

impl<T> IntoBoxedView for T
where
    T: View,
{
    fn as_boxed_view(self) -> Box<AnyView> {
        Box::new(self)
    }
}

impl IntoBoxedView for Box<AnyView> {
    fn as_boxed_view(self) -> Box<AnyView> {
        self
    }
}

impl IntoBoxedView for Box<View> {
    fn as_boxed_view(self) -> Box<AnyView> {
        self.to_any()
    }
}
