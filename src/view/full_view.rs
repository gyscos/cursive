use view::{DimensionRequest, SizeRequest, View, ViewWrapper};
use vec::Vec2;

/// Simple wrapper view that asks for all the space it can get.
pub struct FullView<T: View> {
    view: T,
}

impl<T: View> FullView<T> {
    /// Wraps the given view into a new FullView.
    pub fn new(view: T) -> Self {
        FullView { view: view }
    }
}

impl<T: View> ViewWrapper for FullView<T> {
    wrap_impl!(&self.view);

    fn wrap_get_min_size(&self, req: SizeRequest) -> Vec2 {
        let w = match req.w {
            DimensionRequest::Fixed(w) |
            DimensionRequest::AtMost(w) => w,
            DimensionRequest::Unknown => self.view.get_min_size(req).x,
        };
        let h = match req.h {
            DimensionRequest::Fixed(h) |
            DimensionRequest::AtMost(h) => h,
            DimensionRequest::Unknown => self.view.get_min_size(req).y,
        };

        Vec2::new(w, h)
    }
}
