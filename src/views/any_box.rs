use std::ops::{Deref, DerefMut};
use view::{AnyView, BoxableView, ViewWrapper};

/// A boxed `AnyView`.
pub struct AnyBox {
    view: Box<AnyView>,
}

impl AnyBox {
    /// Creates a new `AnyBox` around the given boxed view.
    pub fn new(view: Box<AnyView>) -> Self {
        AnyBox { view }
    }

    /// Box the given view
    pub fn boxed<T: BoxableView>(view: T) -> Self {
        AnyBox::new(view.as_boxed_view())
    }

    /// Returns the inner boxed view.
    pub fn unwrap(self) -> Box<AnyView> {
        self.view
    }
}

impl Deref for AnyBox {
    type Target = AnyView;

    fn deref(&self) -> &AnyView {
        &*self.view
    }
}

impl DerefMut for AnyBox {
    fn deref_mut(&mut self) -> &mut AnyView {
        &mut *self.view
    }
}

impl ViewWrapper for AnyBox {
    type V = AnyView;

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
