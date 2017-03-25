use view::View;
use views::{IdView, RefCellView};

/// Makes a view wrappable in an [`IdView`].
///
/// [`IdView`]: ../views/struct.IdView.html
pub trait Identifiable: View + Sized {
    /// Wraps this view into an `IdView` with the given id.
    ///
    /// This is just a shortcut for `IdView::new(id, self)`
    fn with_id(self, id: &str) -> IdView<Self> {
        IdView::new(id, self)
    }

    /// Wraps this view into both a [`RefCellView`] and an `IdView`.
    ///
    /// This allows to call [`Cursive::find_id_mut`].
    ///
    /// [`RefCellView`]: ../views/struct.RefCellView.html
    /// [`Cursive::find_id_mut`]: ../struct.Cursive.html#method.find_id_mut
    fn with_id_mut(self, id: &str) -> IdView<RefCellView<Self>> {
        RefCellView::new(self).with_id(id)
    }
}

/// Any `View` implements this trait.
impl<T: View> Identifiable for T {}
