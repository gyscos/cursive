use crate::{view::ViewWrapper, Vec2, View};

/// View wrapper overriding the `View::layout` method.
pub struct OnLayoutView<V> {
    view: V,
    on_layout: Box<dyn FnMut(&mut V, Vec2)>,
}

impl<V> OnLayoutView<V> {
    /// Wraps a view in an `OnLayoutView`.
    ///
    /// Will run the given closure for layout _instead_ of the one from `view`.
    ///
    /// ```rust
    /// use cursive_core::{View, views::{TextView, OnLayoutView}};
    ///
    /// let view = TextView::new("foo");
    ///
    /// // Here we just run the innver view's layout.
    /// OnLayoutView::new(view, |v, s| v.layout(s));
    /// ```
    pub fn new<F>(view: V, on_layout: F) -> Self
    where
        F: FnMut(&mut V, Vec2) + 'static,
    {
        let on_layout = Box::new(on_layout);
        OnLayoutView { view, on_layout }
    }

    /// Wraps a view in an `OnLayoutView`.
    ///
    /// This is a shortcut for `Self::new(view, V::layout)`
    ///
    /// You can change it later with `set_on_layout`.
    pub fn wrap(view: V) -> Self
    where
        V: View,
    {
        Self::new(view, V::layout)
    }

    /// Replaces the callback to run.
    pub fn set_on_layout<F>(&mut self, on_layout: F)
    where
        F: FnMut(&mut V, Vec2) + 'static,
    {
        self.on_layout = Box::new(on_layout);
    }

    inner_getters!(self.view: V);
}

impl<V: View> ViewWrapper for OnLayoutView<V> {
    wrap_impl!(self.view: V);

    fn wrap_layout(&mut self, size: Vec2) {
        (self.on_layout)(&mut self.view, size);
    }
}
