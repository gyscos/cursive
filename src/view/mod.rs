//! Defines various views to use when creating the layout.

mod box_view;
mod stack_view;
mod text_view;
mod key_event_view;
mod view_path;
mod dialog;
mod button;
mod sized_view;
mod view_wrapper;

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

use event::EventResult;
use vec::{Vec2,ToVec2};
use printer::Printer;

/// Describe constraints on a view layout in one dimension.
#[derive(PartialEq,Clone,Copy)]
pub enum DimensionRequest {
    /// The view must use exactly the attached size.
    Fixed(u32),
    /// The view is free to choose its size if it stays under the limit.
    AtMost(u32),
    /// No clear restriction apply.
    Unknown,
}

impl DimensionRequest {
    pub fn reduced(self, offset: u32) -> Self {
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
    pub fn reduced<T: ToVec2>(self, offset: T) -> Self {
        let ov = offset.to_vec2();
        SizeRequest {
            w: self.w.reduced(ov.x),
            h: self.h.reduced(ov.y),
        }
    }

    pub fn dummy() -> Self {
        SizeRequest {
            w: DimensionRequest::Unknown,
            h: DimensionRequest::Unknown,
        }
    }
}

/// Main trait defining a view behaviour.
pub trait View {
    /// Called when a key was pressed. Default implementation just ignores it.
    fn on_key_event(&mut self, i32) -> EventResult { EventResult::Ignored }

    /// Returns the minimum size the view requires under the given restrictions.
    fn get_min_size(&self, SizeRequest) -> Vec2 { Vec2::new(1,1) }

    /// Called once the size for this view has been decided, so it can
    /// propagate the information to its children.
    fn layout(&mut self, Vec2) { }

    /// Draws the view within the given bounds.
    fn draw(&self, &Printer);

    /// Finds the view pointed to by the given path.
    /// Returns None if the path doesn't lead to a view.
    fn find(&mut self, &ViewPath) -> Option<&mut Any> { None }
}

