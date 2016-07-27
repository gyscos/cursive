//! Defines various views to use when creating the layout.
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
mod view_path;

// Helper bases
mod scroll;

// Views
mod box_view;
mod button;
mod checkbox;
mod dialog;
mod edit_view;
mod full_view;
mod id_view;
mod key_event_view;
mod linear_layout;
mod list_view;
mod menubar;
mod menu_popup;
mod panel;
mod progress_bar;
mod shadow_view;
mod select_view;
mod sized_view;
mod stack_view;
mod text_view;
mod tracked_view;


use std::any::Any;

use XY;
use direction::Direction;
use event::{Event, EventResult};
use vec::Vec2;
use Printer;

pub use self::position::{Offset, Position};

pub use self::scroll::ScrollBase;

pub use self::id_view::{IdView, Identifiable};
pub use self::box_view::BoxView;
pub use self::button::Button;
pub use self::checkbox::Checkbox;
pub use self::dialog::Dialog;
pub use self::edit_view::EditView;
pub use self::full_view::FullView;
pub use self::key_event_view::KeyEventView;
pub use self::linear_layout::LinearLayout;
pub use self::list_view::ListView;
pub use self::menubar::Menubar;
pub use self::menu_popup::MenuPopup;
pub use self::view_path::ViewPath;
pub use self::panel::Panel;
pub use self::progress_bar::{Counter, ProgressBar, Ticker};
pub use self::select_view::SelectView;
pub use self::shadow_view::ShadowView;
pub use self::sized_view::SizedView;
pub use self::stack_view::StackView;
pub use self::text_view::TextView;
pub use self::tracked_view::TrackedView;
pub use self::view_wrapper::ViewWrapper;


/// Main trait defining a view behaviour.
pub trait View {
    /// Called when a key was pressed. Default implementation just ignores it.
    fn on_event(&mut self, Event) -> EventResult {
        EventResult::Ignored
    }

    /// Returns the minimum size the view requires with the given restrictions.
    ///
    /// If the view is flexible (it has multiple size options), it can try
    /// to return one that fits the given `constraint`.
    /// It's also fine to ignore it and return a fixed value.
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
    /// Returns None if the path doesn't lead to a view.
    fn find(&mut self, &Selector) -> Option<&mut Any> {
        None
    }

    /// This view is offered focus. Will it take it?
    ///
    /// `source` indicates where the focus comes from.
    /// When the source is unclear, `Front` is usually used.
    fn take_focus(&mut self, source: Direction) -> bool {
        let _ = source;
        false
    }
}

/// Dummy view.
///
/// Doesn't print anything. Minimal size is (1,1).
pub struct DummyView;

impl View for DummyView {
    fn draw(&self, _: &Printer) {}
}


/// Cache around a one-dimensional layout result.
///
/// This is not a View, but something to help you if you create your own Views.
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct SizeCache {
    /// Cached value
    pub value: usize,
    /// `true` if the last size was constrained.
    ///
    /// If unconstrained, any request larger than this value
    /// would return the same size.
    pub constrained: bool,
}

impl SizeCache {
    /// Creates a new sized cache
    pub fn new(value: usize, constrained: bool) -> Self {
        SizeCache {
            value: value,
            constrained: constrained,
        }
    }

    /// Returns `true` if `self` is still valid for the given `request`.
    pub fn accept(self, request: usize) -> bool {
        if request < self.value {
            false
        } else if request == self.value {
            true
        } else {
            !self.constrained
        }
    }

    /// Creates a new bi-dimensional cache.
    ///
    /// It will stay valid for the same request, and compatible ones.
    ///
    /// A compatible request is one where, for each axis, either:
    ///
    /// * the request is equal to the cached size, or
    /// * the request is larger than the cached size and the cache is unconstrained
    ///
    /// Notes:
    ///
    /// * `size` must fit inside `req`.
    /// * for each dimension, `constrained = (size == req)`
    fn build(size: Vec2, req: Vec2) -> XY<Self> {
        XY::new(SizeCache::new(size.x, size.x >= req.x),
                SizeCache::new(size.y, size.y >= req.y))
    }
}


/// Selects a single view (if any) in the tree.
pub enum Selector<'a> {
    /// Selects a view from its ID.
    Id(&'a str),
    /// Selects a view from its path.
    Path(&'a ViewPath),
}
