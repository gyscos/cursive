use direction::Direction;
use event::{AnyCb, Event, EventResult};
use rect::Rect;
use std::any::Any;
use vec::Vec2;
use view::{AnyView, Selector};
use Printer;

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
    fn layout(&mut self, Vec2) {}

    /// Returns `true` if the view content changed since last layout phase.
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
    /// If the view is flexible (it has multiple size options), it can try
    /// to return one that fits the given `constraint`.
    /// It's also fine to ignore it and return a fixed value.
    ///
    /// Default implementation always return `(1,1)`.
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        let _ = constraint;
        Vec2::new(1, 1)
    }

    /// Called when an event is received (key press, mouse event, ...).
    ///
    /// You can return an `EventResult`, with an optional callback to be run.
    ///
    /// Default implementation just ignores it.
    fn on_event(&mut self, Event) -> EventResult {
        EventResult::Ignored
    }

    /// Runs a closure on the view identified by the given selector.
    ///
    /// See [`Finder::call_on`] for a nicer interface, implemented for all
    /// views.
    ///
    /// [`Finder::call_on`]: trait.Finder.html#method.call_on
    ///
    /// If the selector doesn't find a match, the closure will not be run.
    ///
    /// Default implementation is a no-op.
    fn call_on_any<'a>(&mut self, _: &Selector, _: AnyCb<'a>) {
        // TODO: FnMut -> FnOnce once it works
    }

    /// Moves the focus to the view identified by the given selector.
    ///
    /// Returns `Ok(())` if the view was found and selected.
    ///
    /// Default implementation simply returns `Err(())`.
    fn focus_view(&mut self, &Selector) -> Result<(), ()> {
        Err(())
    }

    /// This view is offered focus. Will it take it?
    ///
    /// `source` indicates where the focus comes from.
    /// When the source is unclear, `Front` is usually used.
    ///
    /// Default implementation always return `false`.
    fn take_focus(&mut self, source: Direction) -> bool {
        let _ = source;
        false
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
}
