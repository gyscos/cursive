

use owning_ref::{RcRef, OwningHandle};

use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use view::{View, ViewWrapper};

/// Wrapper around a view to provide interior mutability.
pub struct RefCellView<V: View> {
    view: Rc<RefCell<V>>,
}

/// Mutable reference to a view.
pub type ViewRef<V> = OwningHandle<RcRef<RefCell<V>>, RefMut<'static, V>>;

impl<V: View> RefCellView<V> {
    /// Wraps `view` in a new `RefCellView`.
    pub fn new(view: V) -> Self {
        RefCellView { view: Rc::new(RefCell::new(view)) }
    }

    /// Gets mutable access to the inner view.
    pub fn get_mut(&mut self) -> ViewRef<V> {
        // TODO: return a standalone item (not tied to our lifetime)
        // that bundles `self.view.clone()` and allow mutable reference to
        // the inner view.
        let cell_ref = RcRef::new(self.view.clone());

        OwningHandle::new(cell_ref,
                          |x| unsafe { x.as_ref() }.unwrap().borrow_mut())
    }
}

impl<T: View> ViewWrapper for RefCellView<T> {
    type V = T;

    fn with_view<F, R>(&self, f: F) -> Option<R>
        where F: FnOnce(&Self::V) -> R
    {
        self.view.try_borrow().ok().map(|v| f(&*v))
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
        where F: FnOnce(&mut Self::V) -> R
    {
        self.view.try_borrow_mut().ok().map(|mut v| f(&mut *v))
    }
}
