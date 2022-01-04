use crate::view::{View, ViewWrapper};
use crate::views::{NamedView, ViewRef};

/// Provides `call_on<V: View>` to views.
///
/// This trait is mostly a wrapper around [`View::call_on_any`].
///
/// It provides a nicer interface to find a view when you know its type.
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
        V: View,
        F: FnOnce(&mut V) -> R,
    {
        let mut callback = Some(callback);
        let mut result = None;
        self.call_on_all(sel, |v: &mut V| {
            if let Some(callback) = callback.take() {
                result = Some(callback(v));
            }
        });
        result
    }

    /// Runs a callback on all views identified by `sel`.
    ///
    /// Useful if you have multiple views of the same type with the same name.
    fn call_on_all<V, F>(&mut self, sel: &Selector<'_>, callback: F)
    where
        V: View,
        F: FnMut(&mut V);

    /// Convenient method to use `call_on` with a `view::Selector::Name`.
    fn call_on_name<V, F, R>(&mut self, name: &str, callback: F) -> Option<R>
    where
        V: View,
        F: FnOnce(&mut V) -> R,
    {
        self.call_on(&Selector::Name(name), callback)
    }

    /// Convenient method to find a view wrapped in an [`NamedView`].
    fn find_name<V>(&mut self, name: &str) -> Option<ViewRef<V>>
    where
        V: View,
    {
        self.call_on_name(name, NamedView::<V>::get_mut)
    }
}

impl<T: View> Finder for T {
    fn call_on_all<V, F>(&mut self, sel: &Selector<'_>, mut callback: F)
    where
        V: View,
        F: FnMut(&mut V),
    {
        self.call_on_any(sel, &mut |v: &mut dyn View| {
            if let Some(v) = v.downcast_mut::<V>() {
                callback(v);
            } else if let Some(v) = v.downcast_mut::<NamedView<V>>() {
                v.with_view_mut(&mut callback);
            }
        });
    }
}

/// Selects a single view (if any) in the tree.
#[non_exhaustive]
pub enum Selector<'a> {
    /// Selects a view from its name.
    Name(&'a str),
}
