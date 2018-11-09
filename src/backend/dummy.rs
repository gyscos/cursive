//! Dummy backend
use std::thread;

use crossbeam_channel::{self, Receiver, Sender};

use backend;
use event::Event;
use theme;
use vec::Vec2;

/// Dummy backend that does nothing and immediately exits.
///
/// Mostly used for testing.
pub struct Backend {
    inner_sender: Sender<Option<Event>>,
    inner_receiver: Receiver<Option<Event>>,
}

impl Backend {
    /// Creates a new dummy backend.
    pub fn init() -> Box<backend::Backend>
    where
        Self: Sized,
    {
        let (inner_sender, inner_receiver) = crossbeam_channel::bounded(1);
        Box::new(Backend {
            inner_sender,
            inner_receiver,
        })
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

    fn prepare_input(&mut self, _input_request: backend::InputRequest) {
        self.inner_sender.send(Some(Event::Exit));
    }

    fn start_input_thread(
        &mut self, event_sink: Sender<Option<Event>>,
        input_requests: Receiver<backend::InputRequest>,
    ) {
        let receiver = self.inner_receiver.clone();

        thread::spawn(move || {
            for _ in input_requests {
                match receiver.recv() {
                    None => return,
                    Some(event) => event_sink.send(event),
                }
            }
        });
    }

    fn print_at(&self, _: Vec2, _: &str) {}

    fn clear(&self, _: theme::Color) {}

    // This sets the Colours and returns the previous colours
    // to allow you to set them back when you're done.
    fn set_color(&self, colors: theme::ColorPair) -> theme::ColorPair {
        colors
    }

    fn set_effect(&self, _: theme::Effect) {}
    fn unset_effect(&self, _: theme::Effect) {}
}
