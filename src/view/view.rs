use Printer;
use direction::Direction;
use event::{Event, EventResult};
use std::any::Any;
use vec::Vec2;
use view::{AnyView, Selector};

/// Main trait defining a view behaviour.
///
/// This is what you should implement to define a custom View.
pub trait View: Any + AnyView {
    /// Called when a key was pressed.
    ///
    /// Default implementation just ignores it.
    fn on_event(&mut self, Event) -> EventResult {
        EventResult::Ignored
    }

    /// Returns the minimum size the view requires with the given restrictions.
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

    /// Called once the size for this view has been decided,
    ///
    /// View groups should propagate the information to their children.
    fn layout(&mut self, Vec2) {}

    /// Draws the view with the given printer (includes bounds) and focus.
    fn draw(&self, printer: &Printer);

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
    fn call_on_any<'a>(&mut self, _: &Selector, _: Box<FnMut(&mut Any) + 'a>) {
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
}
