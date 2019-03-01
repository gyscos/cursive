use crate::view::{IntoBoxedView, View, ViewWrapper};
use std::ops::{Deref, DerefMut};

/// A boxed `View`.
///
/// It derefs to the wrapped view.
pub struct ViewBox {
    view: Box<dyn View>,
}

impl ViewBox {
    /// Creates a new `ViewBox` around the given boxed view.
    pub fn new(view: Box<dyn View>) -> Self {
        ViewBox { view }
    }

    /// Box the given view
    pub fn boxed<T>(view: T) -> Self
    where
        T: IntoBoxedView,
    {
        ViewBox::new(view.as_boxed_view())
    }

    /// Returns the inner boxed view.
    pub fn unwrap(self) -> Box<dyn View> {
        self.view
    }
}

impl Deref for ViewBox {
    type Target = dyn View;

    fn deref(&self) -> &dyn View {
        &*self.view
    }
}

impl DerefMut for ViewBox {
    fn deref_mut(&mut self) -> &mut dyn View {
        &mut *self.view
    }
}

impl ViewWrapper for ViewBox {
    type V = dyn View;

    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        Some(f(&*self.view))
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        Some(f(&mut *self.view))
    }
}
