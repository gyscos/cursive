//! Defines various views to use when creating the layout.

#[macro_use]
mod view_wrapper;

// Essentials components
mod position;
mod request;
mod view_path;

// Helper bases
mod scroll;

// Views
mod box_view;
mod button;
mod dialog;
mod edit_view;
mod full_view;
mod id_view;
mod key_event_view;
mod linear_layout;
mod shadow_view;
mod select_view;
mod sized_view;
mod stack_view;
mod text_view;
mod tracked_view;


use std::any::Any;

use event::{Event, EventResult};
use vec::Vec2;
use printer::Printer;

pub use self::position::{Position, Offset};

pub use self::request::{DimensionRequest, SizeRequest};
pub use self::scroll::ScrollBase;

pub use self::id_view::IdView;
pub use self::box_view::BoxView;
pub use self::button::Button;
pub use self::dialog::Dialog;
pub use self::edit_view::EditView;
pub use self::full_view::FullView;
pub use self::key_event_view::KeyEventView;
pub use self::linear_layout::LinearLayout;
pub use self::view_path::ViewPath;
pub use self::select_view::SelectView;
pub use self::shadow_view::ShadowView;
pub use self::stack_view::StackView;
pub use self::text_view::TextView;
pub use self::tracked_view::TrackedView;
pub use self::sized_view::SizedView;
pub use self::view_wrapper::ViewWrapper;


/// Main trait defining a view behaviour.
pub trait View {
    /// Called when a key was pressed. Default implementation just ignores it.
    fn on_event(&mut self, Event) -> EventResult {
        EventResult::Ignored
    }

    /// Returns the minimum size the view requires under the given restrictions.
    fn get_min_size(&self, SizeRequest) -> Vec2 {
        Vec2::new(1, 1)
    }

    /// Called once the size for this view has been decided, so it can
    /// propagate the information to its children.
    fn layout(&mut self, Vec2) {}

    /// Draws the view with the given printer (includes bounds) and focus.
    fn draw(&mut self, printer: &Printer);

    /// Finds the view pointed to by the given path.
    /// Returns None if the path doesn't lead to a view.
    fn find(&mut self, &Selector) -> Option<&mut Any> {
        None
    }

    /// This view is offered focus. Will it take it?
    fn take_focus(&mut self) -> bool {
        false
    }
}

/// Selects a single view (if any) in the tree.
pub enum Selector<'a> {
    /// Selects a view from its ID
    Id(&'a str),
    /// Selects a view from its path
    Path(&'a ViewPath),
}
