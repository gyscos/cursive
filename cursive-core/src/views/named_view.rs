use crate::{
    event::{AnyCb, EventResult},
    view::{Selector, View, ViewNotFound, ViewWrapper},
};
use owning_ref::{OwningHandle, RcRef};
use std::cell::{RefCell, RefMut};
use std::ops::DerefMut;
use std::rc::Rc;

/// Wrapper around a view to make it identifiable.
///
/// This lets other views refer to this one using a string identifier.
///
/// See [`Nameable`](crate::view::Nameable) for an easy way to wrap any view with it.
pub struct NamedView<V> {
    view: Rc<RefCell<V>>,
    name: String,
}

/// Mutable reference to a view.
///
/// This behaves like a [`RefMut`], but without being tied to a lifetime.
///
/// [`RefMut`]: std::cell::RefMut
pub type ViewRef<V> = OwningHandle<RcRef<RefCell<V>>, RefMut<'static, V>>;

impl<V> NamedView<V> {
    /// Wraps `view` in a new `NamedView`.
    pub fn new<S: Into<String>>(name: S, view: V) -> Self {
        NamedView {
            view: Rc::new(RefCell::new(view)),
            name: name.into(),
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

    /// Returns the name attached to this view.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Changes the name attached to this view.
    pub fn set_name<S: Into<String>>(&mut self, name: S) {
        self.name = name.into();
    }
}

impl<T: View + 'static> ViewWrapper for NamedView<T> {
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

    fn into_inner(mut self) -> Result<Self::V, Self>
    where
        Self::V: Sized,
    {
        match Rc::try_unwrap(self.view) {
            Err(rc) => {
                // Whoops! Abort! Undo!
                self.view = rc;
                Err(self)
            }
            Ok(cell) => Ok(cell.into_inner()),
        }
    }

    fn wrap_call_on_any<'a>(
        &mut self,
        selector: &Selector<'_>,
        callback: AnyCb<'a>,
    ) {
        match selector {
            &Selector::Name(name) if name == self.name => callback(self),
            s => {
                if let Ok(mut v) = self.view.try_borrow_mut() {
                    v.deref_mut().call_on_any(s, callback);
                }
            }
        }
    }

    fn wrap_focus_view(
        &mut self,
        selector: &Selector<'_>,
    ) -> Result<EventResult, ViewNotFound> {
        match selector {
            &Selector::Name(name) if name == self.name => {
                Ok(EventResult::Consumed(None))
            }
            s => self
                .view
                .try_borrow_mut()
                .map_err(|_| ViewNotFound)
                .and_then(|mut v| v.deref_mut().focus_view(s)),
        }
    }
}
