//! Puppet backend
use crossbeam_channel::{self, Receiver, Sender, TryRecvError};

use self::observed::ObservedCell;
use self::observed::ObservedScreen;
use self::observed::ObservedStyle;
use crate::backend;
use crate::event::Event;
use crate::theme;
use crate::Vec2;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub mod observed;
pub mod observed_screen_view;
mod static_values;

use static_values::*;

/// Puppet backend for testing.
pub struct Backend {
    inner_sender: Sender<Option<Event>>,
    inner_receiver: Receiver<Option<Event>>,
    prev_frame: RefCell<Option<ObservedScreen>>,
    current_frame: RefCell<ObservedScreen>,
    size: Cell<Vec2>,
    current_style: RefCell<Rc<ObservedStyle>>,
    screen_channel: (Sender<ObservedScreen>, Receiver<ObservedScreen>),
}

impl Backend {
    /// Creates new Puppet backend of given or default size.
    pub fn init(size_op: Option<Vec2>) -> Box<Backend>
    where
        Self: Sized,
    {
        let (inner_sender, inner_receiver) = crossbeam_channel::unbounded();
        let size = size_op.unwrap_or(*DEFAULT_SIZE);

        let mut backend = Backend {
            inner_sender,
            inner_receiver,
            prev_frame: RefCell::new(None),
            current_frame: RefCell::new(ObservedScreen::new(size)),
            size: Cell::new(size),
            current_style: RefCell::new(Rc::new(
                DEFAULT_OBSERVED_STYLE.clone(),
            )),
            screen_channel: crossbeam_channel::unbounded(),
        };

        {
            use backend::Backend;
            backend.refresh();
        }

        Box::new(backend)
    }

    /// Returns current ObservedStyle
    pub fn current_style(&self) -> Rc<ObservedStyle> {
        self.current_style.borrow().clone()
    }

    /// Ouput stream of consecutive frames rendered by Puppet backend
    pub fn stream(&self) -> Receiver<ObservedScreen> {
        self.screen_channel.1.clone()
    }

    /// Input stream to inject artificial input to Puppet backend.
    pub fn input(&self) -> Sender<Option<Event>> {
        self.inner_sender.clone()
    }
}

impl backend::Backend for Backend {
    fn poll_event(&mut self) -> Option<Event> {
        match self.inner_receiver.try_recv() {
            Ok(event) => event,
            Err(TryRecvError::Empty) => None,
            Err(e) => panic!("{}", e),
        }
    }

    fn set_title(&mut self, _title: String) {}

    fn refresh(&mut self) {
        let size = self.size.get();
        let current_frame =
            self.current_frame.replace(ObservedScreen::new(size));
        self.prev_frame.replace(Some(current_frame.clone()));
        self.screen_channel.0.send(current_frame).unwrap();
    }

    fn has_colors(&self) -> bool {
        true
    }

    fn screen_size(&self) -> Vec2 {
        self.size.get()
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        let style = self.current_style.borrow().clone();
        let mut screen = self.current_frame.borrow_mut();
        let mut offset: usize = 0;

        for (idx, grapheme) in text.graphemes(true).enumerate() {
            let cpos = pos + Vec2::new(idx + offset, 0);
            screen[cpos] = Some(ObservedCell::new(
                cpos,
                style.clone(),
                Some(grapheme.to_string()),
            ));

            for _ in 0..grapheme.width() - 1 {
                offset += 1;
                let spos = pos + Vec2::new(idx + offset, 0);
                screen[spos] =
                    Some(ObservedCell::new(spos, style.clone(), None));
            }
        }
    }

    fn clear(&self, clear_color: theme::Color) {
        let mut cloned_style = (*self.current_style()).clone();
        let mut screen = self.current_frame.borrow_mut();
        cloned_style.colors.back = clear_color;
        screen.clear(&Rc::new(cloned_style))
    }

    // This sets the Colours and returns the previous colours
    // to allow you to set them back when you're done.
    fn set_color(&self, new_colors: theme::ColorPair) -> theme::ColorPair {
        let mut copied_style = (*self.current_style()).clone();
        let old_colors = copied_style.colors;
        copied_style.colors = new_colors;
        self.current_style.replace(Rc::new(copied_style));

        old_colors
    }

    fn set_effect(&self, effect: theme::Effect) {
        let mut copied_style = (*self.current_style()).clone();
        copied_style.effects.insert(effect);
        self.current_style.replace(Rc::new(copied_style));
    }

    fn unset_effect(&self, effect: theme::Effect) {
        let mut copied_style = (*self.current_style()).clone();
        copied_style.effects.remove(effect);
        self.current_style.replace(Rc::new(copied_style));
    }
}
