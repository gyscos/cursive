use crate::view::View;
use crate::views::IdView;

/// Makes a view wrappable in an [`IdView`].
///
/// [`IdView`]: ../views/struct.IdView.html
pub trait Identifiable: View + Sized {
    /// Wraps this view into an `IdView` with the given id.
    ///
    /// This is just a shortcut for `IdView::new(id, self)`
    ///
    /// You can use the given id to find the view in the layout tree.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::Cursive;
    /// # use cursive::views::TextView;
    /// # use cursive::view::Boxable;
    /// use cursive::view::Identifiable;
    ///
    /// let mut siv = Cursive::dummy();
    /// siv.add_layer(
    ///     TextView::new("foo")
    ///         .with_id("text")
    ///         .fixed_width(10)
    /// );
    ///
    /// // You could call this from an event callback
    /// siv.call_on_id("text", |view: &mut TextView| {
    ///     view.set_content("New content!");
    /// });
    /// ```
    ///
    /// # Notes
    ///
    /// You should call this directly on the view you want to retrieve later,
    /// before other wrappers like [`fixed_width`]. Otherwise, you would be
    /// retrieving a [`BoxView`]!
    ///
    /// [`fixed_width`]: trait.Boxable.html#method.fixed_width
    /// [`BoxView`]: ../views/struct.BoxView.html
    ///
    fn with_id<S: Into<String>>(self, id: S) -> IdView<Self> {
        IdView::new(id, self)
    }
}

/// Any `View` implements this trait.
impl<T: View> Identifiable for T {}
