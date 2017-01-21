//! Base elements required to build views.
//!
//! Views are the main building blocks of your UI.
//!
//! A view can delegate part or all of its responsabilities to child views,
//! forming a view tree. The root of this tree is a `StackView` handled
//! directly by the `Cursive` element.
//!
//! # Layout
//!
//! The layout phase is when the size and location of each view is computed.
//!
//! Each view is given an area of the screen by the `View::layout()` method.
//! With this, the view is free to plan its content, including calling
//! `View::layout()` on its own children.
//!
//! In order to determine how much space should be given each child, parents
//! can use `View::get_min_size()` on them.
//!
//!
//! ### Contracts
//!
//! When building new Views, you should respect these contracts:
//!
//! * By default, `View::layout()` should be called before any call to
//!   `View::draw()` with the same available size. The only exceptions is
//!   when both following conditions are met:
//!     * The available size has not changed since the last call to
//!       `View::layout()`
//!     * `View::needs_relayout()` returns `false`
//!
//!   In this case, it is safe to omit the call to `View::layout()`.
//!
//! * The value returned by `get_min_size` should be an actually viable size,
//!   no matter what the request is. This means calling `View::layout()` with
//!   a size returned by `get_min_size` is **never** an error.

#[macro_use]
mod view_wrapper;

// Essentials components
mod position;
mod size_cache;
mod size_constraint;
mod view_path;

// Helper bases
mod scroll;
mod identifiable;
mod boxable;


use Printer;

use direction::Direction;
use event::{Event, EventResult};
pub use self::boxable::Boxable;
pub use self::identifiable::Identifiable;

pub use self::position::{Offset, Position};

pub use self::scroll::{ScrollBase, ScrollStrategy};

pub use self::size_cache::SizeCache;
pub use self::size_constraint::SizeConstraint;
pub use self::view_path::ViewPath;
pub use self::view_wrapper::ViewWrapper;
use std::any::Any;
use vec::Vec2;


/// Main trait defining a view behaviour.
pub trait View {
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
    fn get_min_size(&mut self, constraint: Vec2) -> Vec2 {
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
    ///     * If they call `get_min_size` or `layout` with stable parameters,
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

    /// Finds the view pointed to by the given path.
    ///
    /// See [`Finder::find`] for a nicer interface, implemented for all views.
    ///
    /// [`Finder::find`]: trait.Finder.html#method.find
    ///
    /// Returns None if the path doesn't lead to a view.
    ///
    /// Default implementation always return `None`.
    fn find_any(&mut self, &Selector) -> Option<&mut Any> {
        None
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

/// Provides `find<V: View>` to views.
///
/// This trait is mostly a wrapper around [`View::find_any`].
///
/// It provides a nicer interface to find a view when you know its type.
///
/// [`View::find_any`]: ./trait.View.html#method.find_any
pub trait Finder {
    /// Tries to find the view pointed to by the given selector.
    ///
    /// If the view is not found, or if it is not of the asked type,
    /// it returns None.
    fn find<V: View + Any>(&mut self, sel: &Selector) -> Option<&mut V>;

    /// Convenient method to use `find` with a `view::Selector::Id`.
    fn find_id<V: View + Any>(&mut self, id: &str) -> Option<&mut V> {
        self.find(&Selector::Id(id))
    }
}

impl<T: View> Finder for T {
    fn find<V: View + Any>(&mut self, sel: &Selector) -> Option<&mut V> {
        self.find_any(sel).and_then(|b| b.downcast_mut::<V>())
    }
}

/// Selects a single view (if any) in the tree.
pub enum Selector<'a> {
    /// Selects a view from its ID.
    Id(&'a str),
    /// Selects a view from its path.
    Path(&'a ViewPath),
}
