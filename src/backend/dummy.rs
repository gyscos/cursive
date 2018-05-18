//! Dummy backend
use backend;
use event;
use theme;
use vec::Vec2;

pub struct Backend;

impl Backend {
    pub fn init() -> Box<backend::Backend>
    where
        Self: Sized,
    {
        Box::new(Backend)
    }
}

impl backend::Backend for Backend {
    fn finish(&mut self) {}

    fn refresh(&mut self) {}

    fn has_colors(&self) -> bool {
        false
    }

    fn screen_size(&self) -> Vec2 {
        (1, 1).into()
    }

    fn poll_event(&mut self) -> event::Event {
        event::Event::Exit
    }

    fn print_at(&self, _: Vec2, _: &str) {}

    fn clear(&self, _: theme::Color) {}

    fn set_refresh_rate(&mut self, _: u32) {}

    // This sets the Colours and returns the previous colours
    // to allow you to set them back when you're done.
    fn set_color(&self, colors: theme::ColorPair) -> theme::ColorPair {
        colors
    }

    fn set_effect(&self, _: theme::Effect) {}
    fn unset_effect(&self, _: theme::Effect) {}
}
