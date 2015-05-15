//! Defines various views to use when creating the layout.

use event::EventResult;

pub use box_view::BoxView;
pub use stack_view::StackView;
pub use text_view::TextView;

use vec2::Vec2;
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

/// Describes constraints on a view layout.
#[derive(PartialEq,Clone,Copy)]
pub struct SizeRequest {
    /// Restriction on the view width
    pub w: DimensionRequest,
    /// Restriction on the view height
    pub h: DimensionRequest,
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
}
