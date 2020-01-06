use crate::view::{View, ViewPath, ViewWrapper};
use crate::views::{NamedView, ViewRef};
use std::any::Any;

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
        V: View + Any,
        F: FnOnce(&mut V) -> R;

    /// Convenient method to use `call_on` with a `view::Selector::Name`.
    fn call_on_name<V, F, R>(&mut self, name: &str, callback: F) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        self.call_on(&Selector::Name(name), callback)
    }

    /// Same as [`call_on_name`](Finder::call_on_name).
    #[deprecated(note = "`call_on_id` is being renamed to `call_on_name`")]
    fn call_on_id<V, F, R>(&mut self, id: &str, callback: F) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        self.call_on_name(id, callback)
    }

    /// Convenient method to find a view wrapped in an [`NamedView`].
    fn find_name<V>(&mut self, name: &str) -> Option<ViewRef<V>>
    where
        V: View + Any,
    {
        self.call_on_name(name, NamedView::<V>::get_mut)
    }

    /// Same as [`find_name`](Finder::find_name()).
    #[deprecated(note = "`find_id` is being renamed to `find_name`")]
    fn find_id<V>(&mut self, id: &str) -> Option<ViewRef<V>>
    where
        V: View + Any,
    {
        self.find_name(id)
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
            let mut callback = |v: &mut dyn Any| {
                if let Some(callback) = callback.take() {
                    if v.is::<V>() {
                        *result_ref =
                            v.downcast_mut::<V>().map(|v| callback(v));
                    } else if v.is::<NamedView<V>>() {
                        *result_ref = v
                            .downcast_mut::<NamedView<V>>()
                            .and_then(|v| v.with_view_mut(callback));
                    }
                }
            };
            self.call_on_any(sel, &mut callback);
        }
        result
    }
}

/// Selects a single view (if any) in the tree.
pub enum Selector<'a> {
    /// Same as [`Selector::Name`].
    #[deprecated(
        note = "`Selector::Id` is being renamed to `Selector::Name`"
    )]
    Id(&'a str),

    /// Selects a view from its name.
    Name(&'a str),

    /// Selects a view from its path.
    Path(&'a ViewPath),
}
