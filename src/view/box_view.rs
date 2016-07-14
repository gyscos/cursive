use std::cmp;

use XY;
use vec::Vec2;
use super::{View, ViewWrapper};

/// Wrapper around another view, with a fixed size.
///
/// Each axis can be enabled independantly.
///
/// * If both axis are fixed, the view always asks for this size.
/// * If both axis are left free, the wrapper has no effect and the underlying
///   view is directly queried.
/// * If only one axis is fixed, it will override the size request when
///   querying the wrapped view.
///
/// # Examples
///
/// ```
/// # use cursive::view::{BoxView,TextView};
/// // Creates a 20x4 BoxView with a TextView content.
/// let view = BoxView::fixed_size((20,4), TextView::new("Hello!"));
/// ```
pub struct BoxView<T: View> {
    size: XY<Option<usize>>,
    view: T,
}

impl<T: View> BoxView<T> {
    /// Wraps `view` in a new `BoxView` with the given size.
    pub fn fixed_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        BoxView::new(Some(size.x), Some(size.y), view)
    }

    /// Creates a new `BoxView` with the given width and height requirements.
    ///
    /// `None` values will use the wrapped view's preferences.
    pub fn new(width: Option<usize>, height: Option<usize>, view: T) -> Self {
        BoxView {
            size: (width, height).into(),
            view: view,
        }
    }

    /// Wraps `view` in a new `BoxView` with fixed width.
    pub fn fixed_width(width: usize, view: T) -> Self {
        BoxView::new(Some(width), None, view)
    }

    /// Wraps `view` in a new `BoxView` with fixed height.
    pub fn fixed_height(height: usize, view: T) -> Self {
        BoxView::new(None, Some(height), view)
    }
}

fn min<T: Ord>(a: T, b: Option<T>) -> T {
    match b {
        Some(b) => cmp::min(a, b),
        None => a,
    }
}

impl<T: View> ViewWrapper for BoxView<T> {
    wrap_impl!(&self.view);

    fn wrap_get_min_size(&mut self, req: Vec2) -> Vec2 {

        if let (Some(w), Some(h)) = self.size.pair() {
            // If we know everything already, no need to ask
            Vec2::new(w, h)
        } else {
            // If req < self.size in any axis, we're screwed.
            // TODO: handle insufficient space
            // (should probably show an error message or a blank canvas)

            // From now on, we assume req >= self.size.

            // Override the request on the restricted axis
            let req = req.zip_map(self.size, min);

            // Back in my time, we didn't ask kids for their opinions!
            let child_size = self.view.get_min_size(req);

            // This calls unwrap_or on each axis
            self.size.unwrap_or(child_size)
        }
    }
}
