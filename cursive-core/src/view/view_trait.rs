use crate::direction::Direction;
use crate::event::{AnyCb, Event, EventResult};
use crate::rect::Rect;
use crate::view::{AnyView, Selector};
use crate::Printer;
use crate::Vec2;
use crate::XY;
use std::any::Any;
use std::ops::RangeInclusive;

/// Error indicating a view was not found.
#[derive(Debug)]
pub struct ViewNotFound;

/// Error indicating a view could not take focus.
#[derive(Debug)]
pub struct CannotFocus;

impl std::fmt::Display for ViewNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "View could not be found")
    }
}

impl std::fmt::Display for CannotFocus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "View does not take focus")
    }
}

impl std::error::Error for ViewNotFound {}

/// A request for size: ranges of acceptable values for X and Y.
pub type SizeRequest = XY<RangeInclusive<usize>>;

impl SizeRequest {
    /// Create a new size request using a single size.
    pub fn simple<T: Into<Vec2>>(size: T) -> Self {
        size.into().map(|x| x..=x)
    }

    /// Creates a new size request with a (0,0) size.
    pub fn empty() -> Self {
        Self::simple((0, 0))
    }

    /// Returns the minimum acceptable size in this request.
    #[must_use]
    pub fn min_size(self) -> Vec2 {
        self.map(|range| *range.start())
    }

    /// Returns the minimum acceptable size in this request.
    #[must_use]
    pub fn max_size(self) -> Vec2 {
        self.map(|range| *range.end())
    }

    /// Add `other` to this size request.
    #[must_use]
    pub fn add<T: Into<Vec2>>(self, other: T) -> Self {
        self.zip_map(other.into(), |range, other| {
            (other + range.start())..=(other + range.end())
        })
    }

    fn max_range<T: Ord>(
        a: RangeInclusive<T>,
        b: RangeInclusive<T>,
    ) -> RangeInclusive<T> {
        use std::cmp::max;
        let (a_start, a_end) = a.into_inner();
        let (b_start, b_end) = b.into_inner();
        max(a_start, b_start)..=max(a_end, b_end)
    }

    fn add_range<T: std::ops::Add<Output = T>>(
        a: RangeInclusive<T>,
        b: RangeInclusive<T>,
    ) -> RangeInclusive<T> {
        let (a_start, a_end) = a.into_inner();
        let (b_start, b_end) = b.into_inner();
        (a_start + b_start)..=(a_end + b_end)
    }

    /// Add the given width to this request.
    #[must_use]
    pub fn add_x(self, width: usize) -> Self {
        self.add_horizontal((width, 0))
    }

    /// Add the given height to this request.
    #[must_use]
    pub fn add_y(self, height: usize) -> Self {
        self.add_vertical((0, height))
    }

    /// Stack the two size requests vertically.
    #[must_use]
    pub fn add_vertical<T: Into<Vec2>>(self, other: T) -> Self {
        // Take the max on X
        // Take the sum on Y
        self.combine_vertical(other.into().map(|x| x..=x))
    }

    /// Stack the two size requests horizontally.
    #[must_use]
    pub fn add_horizontal<T: Into<Vec2>>(self, other: T) -> Self {
        // Take the max on X
        // Take the sum on Y
        self.combine_horizontal(other.into().map(|x| x..=x))
    }

    /// Stack the two size requests vertically.
    #[must_use]
    pub fn combine_vertical(self, other: Self) -> Self {
        // Take the max on X
        // Take the sum on Y
        XY::zip_map_xy(self, other, Self::max_range, Self::add_range)
    }

    /// Stack the two size requests horizontally.
    #[must_use]
    pub fn combine_horizontal(self, other: Self) -> Self {
        // Take the sum on X
        // Take the max on Y
        XY::zip_map_xy(self, other, Self::add_range, Self::max_range)
    }

    /// Remove `other` from this size request.
    #[must_use]
    fn saturating_sub<T: Into<Vec2>>(self, other: T) -> Self {
        self.zip_map(other.into(), |range, other| {
            (range.start().saturating_sub(other))
                ..=(range.end().saturating_sub(other))
        })
    }

    /// Returns the size remaining after taking `self` from `size`.
    ///
    /// Conceptually `size.saturating_sub(self)`
    #[must_use]
    pub fn taken_from(self, size: Vec2) -> Self {
        self.zip_map(size, |range, size| {
            size.saturating_sub(*range.end())
                ..=size.saturating_sub(*range.start())
        })
    }
}

/// Main trait defining a view behaviour.
///
/// This is what you should implement to define a custom View.
pub trait View: Any + AnyView {
    /// Draws the view with the given printer (includes bounds) and focus.
    ///
    /// This is the only *required* method to implement.
    fn draw(&self, printer: &Printer);

    /// Called once the size for this view has been decided.
    ///
    /// It can be used to pre-compute the configuration of child views.
    ///
    /// View groups should propagate the information to their children.
    ///
    /// At this point, the given size is final and cannot be negociated.
    /// It is guaranteed to be the size available for the call to `draw()`.
    fn layout(&mut self, _: Vec2) {}

    /// Should return `true` if the view content changed since the last call
    /// to `layout()`.
    ///
    /// This is mostly an optimisation for views where the layout phase is
    /// expensive.
    ///
    /// * Views can ignore it and always return true (default implementation).
    ///   They will always be assumed to have changed.
    /// * View Groups can ignore it and always re-layout their children.
    ///     * If they call `required_size` or `layout` with stable parameters,
    ///       the children may cache the result themselves and speed up the
    ///       process anyway.
    fn needs_relayout(&self) -> bool {
        true
    }

    /// Returns the minimum size the view requires with the given restrictions.
    ///
    /// This is the main way a view communicate its size to its parent.
    ///
    /// Some views have a fixed size, and will ignore the `constraint`
    /// parameter entirely.
    ///
    /// Some views are flexible, and may adapt fully or partially to the
    /// constraints.
    ///
    /// Default implementation always return `(1,1)`.
    fn required_size(&mut self, constraint: Vec2) -> SizeRequest {
        let _ = constraint;
        Vec2::new(1, 1).into()
    }

    /// Called when an event is received (key press, mouse event, ...).
    ///
    /// You can return an `EventResult`:
    /// * `EventResult::Ignored` means the event was not processed and may be
    ///    sent to another view.
    /// * `EventResult::Consumed` means the event was consumed and should not
    ///    be sent to any other view. It may in addition include a callback
    ///    to be run.
    ///
    /// The default implementation just ignores any event.
    fn on_event(&mut self, _: Event) -> EventResult {
        EventResult::Ignored
    }

    /// Runs a closure on the view identified by the given selector.
    ///
    /// See [`Finder::call_on`] for a nicer interface, implemented for all
    /// views.
    ///
    /// [`Finder::call_on`]: crate::view::Finder::call_on
    ///
    /// If the selector doesn't find a match, the closure will not be run.
    ///
    /// View groups should implement this to forward the call to each children.
    ///
    /// Default implementation is a no-op.
    fn call_on_any<'a>(&mut self, _: &Selector<'_>, _: AnyCb<'a>) {}

    /// Moves the focus to the view identified by the given selector.
    ///
    /// Returns `Ok(_)` if the view was found and selected.
    /// A callback may be included, it should be run on `&mut Cursive`.
    ///
    /// Default implementation simply returns `Err(ViewNotFound)`.
    fn focus_view(
        &mut self,
        _: &Selector<'_>,
    ) -> Result<EventResult, ViewNotFound> {
        Err(ViewNotFound)
    }

    /// Attempt to give this view the focus.
    ///
    /// `source` indicates where the focus comes from.
    /// When the source is unclear (for example mouse events),
    /// `Direction::none()` can be used.
    ///
    /// Returns `Ok(_)` if the focus was taken.
    /// Returns `Err(_)` if this view does not take focus (default implementation).
    fn take_focus(
        &mut self,
        source: Direction,
    ) -> Result<EventResult, CannotFocus> {
        let _ = source;

        Err(CannotFocus)
    }

    /// What part of the view is important and should be visible?
    ///
    /// When only part of this view can be visible, this helps
    /// determine which part.
    ///
    /// It is given the view size (same size given to `layout`).
    ///
    /// Default implementation return the entire view.
    fn important_area(&self, view_size: Vec2) -> Rect {
        Rect::from_size((0, 0), view_size)
    }

    /// Returns the type of this view.
    ///
    /// Useful when you have a `&dyn View`.
    ///
    /// View implementation don't usually have to override this.
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl dyn View {
    /// Attempts to downcast `self` to a concrete type.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    /// Attempts to downcast `self` to a concrete type.
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }

    /// Attempts to downcast `Box<Self>` to a concrete type.
    pub fn downcast<T: Any>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        // Do the check here + unwrap, so the error
        // value is `Self` and not `dyn Any`.
        if self.as_any().is::<T>() {
            Ok(self.as_boxed_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Checks if this view is of type `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.as_any().is::<T>()
    }
}
