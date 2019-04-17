use crate::view::View;
use crate::views::ScrollView;

/// Makes a view wrappable in a [`ScrollView`].
///
/// [`ScrollView`]: crate::views::ScrollView
pub trait Scrollable: View + Sized {
    /// Wraps `self` in a `ScrollView`.
    fn scrollable(self) -> ScrollView<Self> {
        ScrollView::new(self)
    }
}

impl<T: View> Scrollable for T {}
