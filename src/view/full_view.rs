use view::{View, ViewWrapper};
use orientation::Orientation;
use vec::Vec2;

/// Simple wrapper view that asks for all the space it can get.
pub struct FullView<T: View> {
    view: T,
    orientation: Option<Orientation>,
}

impl<T: View> FullView<T> {
    /// Wraps the given view into a new FullView.
    ///
    /// It will always take the entire space available.
    pub fn new(view: T) -> Self {
        FullView {
            view: view,
            orientation: None,
        }
    }

    /// Creates a new wrapper around `view` with full width.
    ///
    /// It will always take the maximum width available.
    pub fn full_width(view: T) -> Self {
        FullView {
            view: view,
            orientation: Some(Orientation::Horizontal),
        }
    }

    /// Creates a new wrapper around `view` with full height.
    ///
    /// It will always take the maximum height available.
    pub fn full_height(view: T) -> Self {
        FullView {
            view: view,
            orientation: Some(Orientation::Vertical),
        }
    }
}

impl<T: View> ViewWrapper for FullView<T> {
    wrap_impl!(&self.view);

    fn wrap_get_min_size(&mut self, mut req: Vec2) -> Vec2 {
        if let Some(orientation) = self.orientation {
            let child_size = self.view.get_min_size(req);
            let orientation = orientation.swap();
            *orientation.get_ref(&mut req) = orientation.get(&child_size);
        }

        req
    }
}
