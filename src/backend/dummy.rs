//! Dummy backend

use crate::backend;
use crate::event::Event;
use crate::theme;
use crate::vec::Vec2;

/// Dummy backend that does nothing and immediately exits.
///
/// Mostly used for testing.
pub struct Backend;

impl Backend {
    /// Creates a new dummy backend.
    pub fn init() -> Box<dyn backend::Backend>
    where
        Self: Sized,
    {
        Box::new(Backend)
    }
}

impl backend::Backend for Backend {
    fn name(&self) -> &str {
        "dummy"
    }

    fn finish(&mut self) {}

    fn refresh(&mut self) {}

    fn has_colors(&self) -> bool {
        false
    }

    fn screen_size(&self) -> Vec2 {
        (1, 1).into()
    }
    fn poll_event(&mut self) -> Option<Event> {
        Some(Event::Exit)
    }

    fn print_at(&self, _: Vec2, _: &str) {}

    fn print_at_rep(&self, _pos: Vec2, _repetitions: usize, _text: &str) {}

    fn clear(&self, _: theme::Color) {}

    // This sets the Colours and returns the previous colours
    // to allow you to set them back when you're done.
    fn set_color(&self, colors: theme::ColorPair) -> theme::ColorPair {
        colors
    }

    fn set_effect(&self, _: theme::Effect) {}
    fn unset_effect(&self, _: theme::Effect) {}
}
