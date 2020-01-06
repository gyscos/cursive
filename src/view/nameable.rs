use crate::view::View;
use crate::views::NamedView;

/// Makes a view wrappable in an [`NamedView`].
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
    ///         .with_name("text")
    ///         .fixed_width(10)
    /// );
    ///
    /// // You could call this from an event callback
    /// siv.call_on_name("text", |view: &mut TextView| {
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
    /// [`fixed_width`]: crate::view::Resizable::fixed_width
    /// [`ResizedView`]: crate::views::ResizedView
    ///
    fn with_name<S: Into<String>>(self, name: S) -> NamedView<Self> {
        NamedView::new(name, self)
    }

    /// Same as [`with_name`](Nameable::with_name)
    #[deprecated(note = "`with_id` is being renamed to `with_name`")]
    fn with_id<S: Into<String>>(self, id: S) -> NamedView<Self> {
        self.with_name(id)
    }
}

/// Any `View` implements this trait.
impl<T: View> Nameable for T {}
