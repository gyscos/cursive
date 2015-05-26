//! Defines various views to use when creating the layout.

#[macro_use] mod view_wrapper;
mod box_view;
mod stack_view;
mod text_view;
mod key_event_view;
mod view_path;
mod dialog;
mod button;
mod sized_view;
mod full_view;
mod id_view;
mod shadow_view;
mod edit_view;

use std::any::Any;

pub use self::view_path::ViewPath;
pub use self::key_event_view::KeyEventView;
pub use self::box_view::BoxView;
pub use self::stack_view::StackView;
pub use self::text_view::TextView;
pub use self::dialog::Dialog;
pub use self::button::Button;
pub use self::sized_view::SizedView;
pub use self::view_wrapper::ViewWrapper;
pub use self::full_view::FullView;
pub use self::id_view::IdView;
pub use self::shadow_view::ShadowView;
pub use self::edit_view::EditView;

use event::EventResult;
use vec::{Vec2,ToVec2};
use printer::Printer;

/// Main trait defining a view behaviour.
pub trait View {
    /// Called when a key was pressed. Default implementation just ignores it.
    fn on_key_event(&mut self, i32) -> EventResult { EventResult::Ignored }

    /// Returns the minimum size the view requires under the given restrictions.
    fn get_min_size(&self, SizeRequest) -> Vec2 { Vec2::new(1,1) }

    /// Called once the size for this view has been decided, so it can
    /// propagate the information to its children.
    fn layout(&mut self, Vec2) { }

    /// Draws the view with the given printer (includes bounds) and focus.
    fn draw(&mut self, printer: &Printer, focused: bool);

    /// Finds the view pointed to by the given path.
    /// Returns None if the path doesn't lead to a view.
    fn find(&mut self, &Selector) -> Option<&mut Any> { None }

    /// This view is offered focus. Will it take it?
    fn take_focus(&mut self) -> bool { false }
}

/// Selects a single view (if any) in the tree.
pub enum Selector<'a> {
    Id(&'a str),
    Path(&'a ViewPath),
}

/// Describe constraints on a view layout in one dimension.
#[derive(PartialEq,Clone,Copy)]
pub enum DimensionRequest {
    /// The view must use exactly the attached size.
    Fixed(usize),
    /// The view is free to choose its size if it stays under the limit.
    AtMost(usize),
    /// No clear restriction apply.
    Unknown,
}

impl DimensionRequest {
    /// Returns a new request, reduced from the original by the given offset.
    pub fn reduced(self, offset: usize) -> Self {
        match self {
            DimensionRequest::Fixed(w) => DimensionRequest::Fixed(w - offset),
            DimensionRequest::AtMost(w) => DimensionRequest::AtMost(w - offset),
            DimensionRequest::Unknown => DimensionRequest::Unknown,
        }
    }
}

/// Describes constraints on a view layout.
#[derive(PartialEq,Clone,Copy)]
pub struct SizeRequest {
    /// Restriction on the view width
    pub w: DimensionRequest,
    /// Restriction on the view height
    pub h: DimensionRequest,
}

impl SizeRequest {
    /// Returns a new SizeRequest, reduced from the original by the given offset.
    pub fn reduced<T: ToVec2>(self, offset: T) -> Self {
        let ov = offset.to_vec2();
        SizeRequest {
            w: self.w.reduced(ov.x),
            h: self.h.reduced(ov.y),
        }
    }

    /// Creates a new dummy request, with no restriction.
    pub fn dummy() -> Self {
        SizeRequest {
            w: DimensionRequest::Unknown,
            h: DimensionRequest::Unknown,
        }
    }
}


