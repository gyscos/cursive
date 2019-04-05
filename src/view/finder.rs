use crate::view::{View, ViewPath, ViewWrapper};
use crate::views::{IdView, ViewRef};
use std::any::Any;

/// Provides `call_on<V: View>` to views.
///
/// This trait is mostly a wrapper around [`View::call_on_any`].
///
/// It provides a nicer interface to find a view when you know its type.
///
/// [`View::call_on_any`]: ./trait.View.html#method.call_on_any
pub trait Finder {
    /// Runs a callback on the view identified by `sel`.
    ///
    /// If the view is found, return the result of `callback`.
    ///
    /// If the view is not found, or if it is not of the asked type,
    /// it returns `None`.
    fn call_on<V, F, R>(
        &mut self,
        sel: &Selector<'_>,
        callback: F,
    ) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R;

    /// Convenient method to use `call_on` with a `view::Selector::Id`.
    fn call_on_id<V, F, R>(&mut self, id: &str, callback: F) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        self.call_on(&Selector::Id(id), callback)
    }

    /// Convenient method to find a view wrapped in an [`IdView`].
    ///
    /// [`IdView`]: views/struct.IdView.html
    fn find_id<V>(&mut self, id: &str) -> Option<ViewRef<V>>
    where
        V: View + Any,
    {
        self.call_on_id(id, IdView::<V>::get_mut)
    }
}

impl<T: View> Finder for T {
    fn call_on<V, F, R>(
        &mut self,
        sel: &Selector<'_>,
        callback: F,
    ) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        let mut result = None;
        {
            let result_ref = &mut result;

            let mut callback = Some(callback);
            let callback = |v: &mut dyn Any| {
                if let Some(callback) = callback.take() {
                    if v.is::<V>() {
                        *result_ref =
                            v.downcast_mut::<V>().map(|v| callback(v));
                    } else if v.is::<IdView<V>>() {
                        *result_ref = v
                            .downcast_mut::<IdView<V>>()
                            .and_then(|v| v.with_view_mut(callback));
                    }
                }
            };
            self.call_on_any(sel, Box::new(callback));
        }
        result
    }
}

/// Selects a single view (if any) in the tree.
pub enum Selector<'a> {
    /// Selects a view from its ID.
    Id(&'a str),
    /// Selects a view from its path.
    Path(&'a ViewPath),
}
