use std::any::Any;
use view::{View, ViewPath, ViewWrapper};
use views::IdView;

/// Provides `call_on<V: View>` to views.
///
/// This trait is mostly a wrapper around [`View::call_on_any`].
///
/// It provides a nicer interface to find a view when you know its type.
///
/// [`View::call_on_any`]: ./trait.View.html#method.call_on_any
pub trait Finder {
    /// Tries to find the view pointed to by the given selector.
    ///
    /// If the view is not found, or if it is not of the asked type,
    /// it returns None.
    fn call_on<V, F, R>(&mut self, sel: &Selector, callback: F) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R;

    /// Convenient method to use `call_on` with a `view::Selector::Id`.
    fn find_id<V, F, R>(&mut self, id: &str, callback: F) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        self.call_on(&Selector::Id(id), callback)
    }
}

impl<T: View> Finder for T {
    fn call_on<V, F, R>(&mut self, sel: &Selector, callback: F) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        let mut result = None;
        {
            let result_ref = &mut result;

            let mut callback = Some(callback);
            let callback = |v: &mut Any| {
                if let Some(callback) = callback.take() {
                    if v.is::<V>() {
                        *result_ref =
                            v.downcast_mut::<V>().map(|v| callback(v));
                    } else if v.is::<IdView<V>>() {
                        *result_ref = v.downcast_mut::<IdView<V>>()
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
