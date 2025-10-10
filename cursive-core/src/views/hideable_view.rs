use crate::event::AnyCb;
use crate::view::{Selector, View, ViewWrapper};
use crate::Vec2;
use crate::With;

/// Wrapper around another view that can be hidden at will.
///
/// By default, it simply forwards all calls to the inner view.
///
/// When hidden (with `HideableView::hide()`), it will appear as a zero-sized
/// invisible view, will not take focus and will not accept input.
///
/// It can be made visible again with `HideableView::unhide()`.
pub struct HideableView<V> {
    view: V,
    visible: bool,
    invalidated: bool,
}

new_default!(HideableView<V: Default>);

impl<V> HideableView<V> {
    /// Creates a new HideableView around `view`.
    ///
    /// It will be visible by default.
    pub fn new(view: V) -> Self {
        HideableView {
            view,
            visible: true,
            invalidated: true,
        }
    }

    /// Sets the visibility for this view.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        self.invalidate();
    }

    /// Sets the visibility for this view to `false`.
    pub fn hide(&mut self) {
        self.set_visible(false);
    }

    /// Sets the visibility for this view to `true`.
    pub fn unhide(&mut self) {
        self.set_visible(true);
    }

    /// Sets the visibility for this view to `false`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn hidden(self) -> Self {
        self.with(Self::hide)
    }

    /// Sets the visibility for this view to flag value.
    /// Useful when creating views needs to be decided at runtime.
    /// Chainable variant.
    #[must_use]
    pub fn visible(self, flag: bool) -> Self {
        self.with(|s| s.set_visible(flag))
    }

    /// Returns `true` if the wrapped view is going to be visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    fn invalidate(&mut self) {
        self.invalidated = true;
    }

    inner_getters!(self.view: V);
}

impl<V: View> ViewWrapper for HideableView<V> {
    type V = V;

    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        if self.visible {
            Some(f(&self.view))
        } else {
            None
        }
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        if self.visible {
            Some(f(&mut self.view))
        } else {
            None
        }
    }

    fn wrap_call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        // We always run callbacks, even when invisible.
        self.view.call_on_any(selector, callback)
    }

    fn into_inner(self) -> Result<Self::V, Self>
    where
        Self: Sized,
        Self::V: Sized,
    {
        Ok(self.view)
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.invalidated = false;
        self.with_view_mut(|v| v.layout(size));
    }

    fn wrap_needs_relayout(&self) -> bool {
        self.invalidated || (self.visible && self.view.needs_relayout())
    }
}

#[crate::blueprint(HideableView::new(view))]
struct Blueprint {
    view: crate::views::BoxedView,
    visible: Option<bool>,
}

crate::manual_blueprint!(with hideable, |config, context| {
    let visible: Option<bool> = context.resolve(&config["visible"])?;

    Ok(move |view| HideableView::new(view).visible(visible.unwrap_or(true)))
});
