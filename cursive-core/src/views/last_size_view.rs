use crate::view::View;
use crate::view::ViewWrapper;
use crate::Vec2;

/// Wrapper around a view that remembers its size.
pub struct LastSizeView<T> {
    /// Wrapped view.
    pub view: T,
    /// Cached size from the last layout() call.
    pub size: Vec2,
}

new_default!(LastSizeView<V: Default>);

impl<T> LastSizeView<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        LastSizeView {
            view,
            size: Vec2::zero(),
        }
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for LastSizeView<T> {
    wrap_impl!(self.view: T);

    fn wrap_layout(&mut self, size: Vec2) {
        self.size = size;
        self.view.layout(size);
    }
}

#[crate::blueprint(LastSizeView::new(view))]
struct Blueprint {
    view: crate::views::BoxedView,
}

crate::manual_blueprint!(with last_size, |_, _| Ok(LastSizeView::new));
