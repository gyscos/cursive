use crate::view::View;
use crate::views::NamedView;

/// Makes a view wrappable in an [`NamedView`].
///
/// [`NamedView`]: ../views/struct.NamedView.html
pub trait Nameable: View + Sized {
    /// Wraps this view into an `NamedView` with the given id.
    ///
    /// This is just a shortcut for `NamedView::new(id, self)`
    ///
    /// You can use the given id to find the view in the layout tree.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::Cursive;
    /// # use cursive::views::TextView;
    /// # use cursive::view::Resizable;
    /// use cursive::view::Nameable;
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
    /// retrieving a [`ResizedView`]!
    ///
    /// [`fixed_width`]: trait.Resizable.html#method.fixed_width
    /// [`ResizedView`]: ../views/struct.ResizedView.html
    ///
    fn with_name<S: Into<String>>(self, name: S) -> NamedView<Self> {
        NamedView::new(name, self)
    }

    /// Same as [`with_name`](Self::with_name())
    #[deprecated(note = "with_id is being renamed to with_name")]
    fn with_id<S: Into<String>>(self, id: S) -> NamedView<Self> {
        self.with_name(id)
    }
}

/// Any `View` implements this trait.
impl<T: View> Nameable for T {}
