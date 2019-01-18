#![warn(missing_docs)]

use std::thread;

use crossbeam_channel::{self, Receiver, Sender};

use backend;
use backend::puppet::observed::ObservedCell;
use backend::puppet::observed::ObservedScreen;
use backend::puppet::observed::ObservedStyle;
use event::Event;
use std::cell::Cell;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashSet;
use std::rc::Rc;
use theme;
use theme::Color;
use theme::ColorPair;
use theme::Style;
use unicode_segmentation::UnicodeSegmentation;
use vec::Vec2;
use XY;

pub mod observed;

pub const DEFAULT_SIZE: Vec2 = XY::<usize> { x: 120, y: 80 };
pub const DEFAULT_OBSERVED_STYLE: ObservedStyle = ObservedStyle {
    colors: ColorPair {
        front: Color::TerminalDefault,
        back: Color::TerminalDefault,
    },
    effects: enum_set!(),
};

#[derive(Debug, Clone)]
pub struct PuppetBackendState {
    prev_frame: Option<ObservedScreen>,
    current_frame: Option<ObservedScreen>,
    size: Vec2,
    current_style: Rc<ObservedStyle>,
}

impl PuppetBackendState {
    pub fn new() -> Self {
        Self::new_with_size(DEFAULT_SIZE)
    }

    pub fn new_with_size(size : Vec2) -> Self {
        PuppetBackendState {
            prev_frame: None,
            current_frame: None,
            size,
            current_style: Rc::new(DEFAULT_OBSERVED_STYLE),
        }
    }
}

pub struct Backend {
    inner_sender: Sender<Option<Event>>,
    inner_receiver: Receiver<Option<Event>>,
    state: RefCell<PuppetBackendState>,
}

impl Backend {
    pub fn init() -> Box<backend::Backend>
    where
        Self: Sized,
    {
        let (inner_sender, inner_receiver) = crossbeam_channel::bounded(1);

        Box::new(Backend {
            inner_sender,
            inner_receiver,
            state: RefCell::new(PuppetBackendState::new()),
        })
    }

    pub fn current_frame(&self) -> Option<Ref<ObservedScreen>> {
        let is_frame = self.state.borrow().current_frame.is_some();
        if is_frame {
            Some(Ref::map(self.state.borrow(), |state| {
                state.current_frame.as_ref().unwrap()
            }))
        } else {
            None
        }
    }

    pub fn current_style(&self) -> Rc<ObservedStyle> {
        self.state.borrow().current_style.clone()
    }

    fn current_frame_mut(&self) -> Option<RefMut<ObservedScreen>> {
        let is_frame = self.state.borrow().current_frame.is_some();
        if is_frame {
            Some(RefMut::map(self.state.borrow_mut(), |state| {
                state.current_frame.as_mut().unwrap()
            }))
        } else {
            None
        }
    }
}

impl backend::Backend for Backend {
    fn finish(&mut self) {}

    fn start_input_thread(
        &mut self, event_sink: Sender<Option<Event>>,
        input_requests: Receiver<backend::InputRequest>,
    ) {
        let receiver = self.inner_receiver.clone();

        thread::spawn(move || {
            for _ in input_requests {
                match receiver.recv() {
                    Err(_) => return,
                    Ok(event) => {
                        if event_sink.send(event).is_err() {
                            return;
                        }
                    }
                }
            }
        });
    }

    fn prepare_input(&mut self, _input_request: backend::InputRequest) {
        self.inner_sender.send(Some(Event::Exit)).unwrap();
    }

    fn refresh(&mut self) {
        let mut state = self.state.borrow_mut();
        state.prev_frame = state.current_frame.take();
        state.current_frame = Some(ObservedScreen::new(state.size));
    }

    fn has_colors(&self) -> bool {
        true
    }

    fn screen_size(&self) -> Vec2 {
        let state = self.state.borrow();
        state.size
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        let state = self.state.borrow();

        let mut skip: usize = 0;
        //since some graphemes are visually longer than one char, we need to track printer offset.
        let mut offset: usize = 0;

        let mut screen = self.current_frame_mut().unwrap();
        let style = self.current_style();

        'printer: for (idx, c) in text.graphemes(true).enumerate() {
            while skip > 0 {
                screen[&Vec2::new(pos.x + offset, pos.y)] =
                    Some(ObservedCell::new(style.clone(), None));

                skip -= 1;
                offset += 1;
                continue 'printer;
            }
        }
    }

    fn clear(&self, clear_color: theme::Color) {
        let mut screen = self.current_frame_mut().unwrap();
        let mut cloned_style = (*self.current_style()).clone();
        cloned_style.colors.back = clear_color;
        screen.clear(&Rc::new(cloned_style))
    }

    // This sets the Colours and returns the previous colours
    // to allow you to set them back when you're done.
    fn set_color(&self, new_colors: theme::ColorPair) -> theme::ColorPair {
        let mut copied_style = (*self.current_style()).clone();
        let old_colors = copied_style.colors;
        copied_style.colors = new_colors;
        self.state.borrow_mut().current_style = Rc::new(copied_style);

        old_colors
    }

    fn set_effect(&self, effect: theme::Effect) {
        let mut copied_style = (*self.current_style()).clone();
        copied_style.effects.insert(effect);
        self.state.borrow_mut().current_style = Rc::new(copied_style);
    }

    fn unset_effect(&self, effect: theme::Effect) {
        let mut copied_style = (*self.current_style()).clone();
        copied_style.effects.remove(effect);
        self.state.borrow_mut().current_style = Rc::new(copied_style);
    }
}
