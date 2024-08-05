use crate::{view::ViewWrapper, Vec2, View};

type Callback<V> = dyn FnMut(&mut V, Vec2) + Send + Sync;

/// View wrapper overriding the `View::layout` method.
pub struct OnLayoutView<V> {
    view: V,
    on_layout: Box<Callback<V>>,
}

impl<V: 'static> OnLayoutView<V> {
    /// Wraps a view in an `OnLayoutView`.
    ///
    /// Will run the given closure for layout _instead_ of the one from `view`.
    ///
    /// ```rust
    /// use cursive_core::{
    ///     views::{OnLayoutView, TextView},
    ///     View,
    /// };
    ///
    /// let view = TextView::new("foo");
    ///
    /// // Here we just run the innver view's layout.
    /// OnLayoutView::new(view, |v, s| v.layout(s));
    /// ```
    pub fn new<F>(view: V, on_layout: F) -> Self
    where
        F: FnMut(&mut V, Vec2) + 'static + Send + Sync,
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
    #[crate::callback_helpers]
    pub fn set_on_layout<F>(&mut self, on_layout: F)
    where
        F: FnMut(&mut V, Vec2) + 'static + Send + Sync,
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

#[crate::blueprint(OnLayoutView::wrap(view))]
struct Blueprint {
    view: crate::views::BoxedView,

    on_layout: Option<_>,
}

crate::manual_blueprint!(with on_layout, |config, context| {
    let callback = context.resolve(config)?;
    Ok(move |view| {
        let mut view = OnLayoutView::wrap(view);
        if let Some(callback) = callback {
            view.set_on_layout_cb(callback);
        }
        view
    })
});
