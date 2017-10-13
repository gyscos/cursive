use owning_ref::{OwningHandle, RcRef};
use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use view::{Selector, View, ViewWrapper};

/// Wrapper around a view to provide interior mutability.
pub struct IdView<V: View> {
    view: Rc<RefCell<V>>,
    id: String,
}

/// Mutable reference to a view.
///
/// This behaves like a [`RefMut`], but without being tied to a lifetime.
///
/// [`RefMut`]: https://doc.rust-lang.org/std/cell/struct.RefMut.html
pub type ViewRef<V> = OwningHandle<RcRef<RefCell<V>>, RefMut<'static, V>>;

impl<V: View> IdView<V> {
    /// Wraps `view` in a new `IdView`.
    pub fn new<S: Into<String>>(id: S, view: V) -> Self {
        IdView {
            view: Rc::new(RefCell::new(view)),
            id: id.into(),
        }
    }

    /// Gets mutable access to the inner view.
    ///
    /// This returns a `ViewRef<V>`, which implement `DerefMut<Target = V>`.
    ///
    /// # Panics
    ///
    /// Panics if another reference for this view already exists.
    pub fn get_mut(&mut self) -> ViewRef<V> {
        let cell_ref = RcRef::new(Rc::clone(&self.view));

        OwningHandle::new_mut(cell_ref)
    }
}

// Shortcut for a boxed callback (for the wrap_call_on_any method).
type BoxedCallback<'a> = Box<for<'b> FnMut(&'b mut Any) + 'a>;

impl<T: View + 'static> ViewWrapper for IdView<T> {
    type V = T;

    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        self.view.try_borrow().ok().map(|v| f(&*v))
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        self.view.try_borrow_mut().ok().map(|mut v| f(&mut *v))
    }

    // Some for<'b> weirdness here to please the borrow checker gods...
    fn wrap_call_on_any<'a>(
        &mut self, selector: &Selector, mut callback: BoxedCallback<'a>
    ) {
        match selector {
            &Selector::Id(id) if id == self.id => callback(self),
            s => {
                self.view
                    .try_borrow_mut()
                    .ok()
                    .map(|mut v| v.call_on_any(s, callback));
            }
        }
    }

    fn wrap_focus_view(&mut self, selector: &Selector) -> Result<(), ()> {
        match selector {
            &Selector::Id(id) if id == self.id => Ok(()),
            s => self.view
                .try_borrow_mut()
                .map_err(|_| ())
                .and_then(|mut v| v.focus_view(s)),
        }
    }
}
