use view::View;
use views::IdView;

/// Makes a view wrappable in an [`IdView`].
///
/// [`IdView`]: ../views/struct.IdView.html
pub trait Identifiable: View + Sized {
    /// Wraps this view into an `IdView` with the given id.
    ///
    /// This is just a shortcut for `IdView::new(id, self)`
    fn with_id<S: Into<String>>(self, id: S) -> IdView<Self> {
        IdView::new(id, self)
    }
}

/// Any `View` implements this trait.
impl<T: View> Identifiable for T {}
