use crate::event::{Event, EventResult};
use crate::rect::Rect;

use crate::vec::Vec2;
use crate::view::View;

/// Inner implementation for `ScrollCore::on_event`
pub trait InnerOnEvent {
    /// Performs `View::on_event()`
    fn on_event(&mut self, event: Event) -> EventResult;

    /// Performs `View::important_area()`
    fn important_area(&self, size: Vec2) -> Rect;
}

impl<'a, V: View> InnerOnEvent for &'a mut V {
    fn on_event(&mut self, event: Event) -> EventResult {
        <V as View>::on_event(self, event)
    }
    fn important_area(&self, size: Vec2) -> Rect {
        <V as View>::important_area(self, size)
    }
}

/// Inner implementation for `ScrollCore::draw()`
/// Inner implementation for `ScrollCore::InnerLayout()`
pub trait InnerLayout {
    /// Performs `View::layout()`
    fn layout(&mut self, size: Vec2);
    /// Performs `View::needs_relayout()`
    fn needs_relayout(&self) -> bool;
    /// Performs `View::required_size()`
    fn required_size(&mut self, constraint: Vec2) -> Vec2;
}

impl<'a, V: View> InnerLayout for &'a mut V {
    fn layout(&mut self, size: Vec2) {
        <V as View>::layout(self, size);
    }
    fn needs_relayout(&self) -> bool {
        <V as View>::needs_relayout(self)
    }
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        <V as View>::required_size(self, constraint)
    }
}

/// Inner implementation for `ScrollCore::required_size()`
pub trait InnerRequiredSize {
    /// Performs `View::needs_relayout()`
    fn needs_relayout(&self) -> bool;
    /// Performs `View::required_size()`
    fn required_size(&mut self, constraint: Vec2) -> Vec2;
}

impl<V: View> InnerRequiredSize for &mut V {
    fn needs_relayout(&self) -> bool {
        <V as View>::needs_relayout(self)
    }
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        <V as View>::required_size(self, constraint)
    }
}
