use views::IdView;
use view::View;

/// Makes a view wrappable in an [`IdView`].
///
/// [`IdView`]: ../views/struct.IdView.html
pub trait Identifiable: View + Sized {
    /// Wraps this view into an IdView with the given id.
    fn with_id(self, id: &str) -> IdView<Self> {
        IdView::new(id, self)
    }
}

impl<T: View> Identifiable for T {}
