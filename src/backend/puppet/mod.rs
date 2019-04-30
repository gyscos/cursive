//! Puppet backend

#![warn(missing_docs)]
#![allow(warnings)]
#![warn(unused)]
#![allow(bad_style)]

use std::thread;

use crossbeam_channel::{self, Receiver, Sender, TryRecvError};

use crate::backend;
use crate::backend::puppet::observed::ObservedCell;
use crate::backend::puppet::observed::ObservedScreen;
use crate::backend::puppet::observed::ObservedStyle;
use crate::event::Event;
use std::cell::RefCell;
use std::rc::Rc;
use crate::theme;
use crate::theme::{Color, Effect};
use crate::theme::ColorPair;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use crate::vec::Vec2;
use crate::XY;

#[macro_use]
use lazy_static::lazy_static;

use enumset::EnumSet;

pub mod observed;
pub mod observed_screen_view;

lazy_static! {
    pub static ref DEFAULT_SIZE: Vec2 = XY::<usize> { x: 120, y: 80 };
    pub static ref DEFAULT_OBSERVED_STYLE: ObservedStyle = ObservedStyle {
        colors: ColorPair {
            front: Color::TerminalDefault,
            back: Color::TerminalDefault,
        },
        effects: EnumSet::<Effect>::empty(),
    };
}

pub struct Backend {
    inner_sender: Sender<Option<Event>>,
    inner_receiver: Receiver<Option<Event>>,
    prev_frame: RefCell<Option<ObservedScreen>>,
    current_frame: RefCell<ObservedScreen>,
    size: RefCell<Vec2>,
    current_style: RefCell<Rc<ObservedStyle>>,
    screen_channel: (Sender<ObservedScreen>, Receiver<ObservedScreen>),
}

impl Backend {
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
            size: RefCell::new(size),
            current_style: RefCell::new(Rc::new(DEFAULT_OBSERVED_STYLE.clone())),
            screen_channel: crossbeam_channel::unbounded(),
        };

        {
            use backend::Backend;
            backend.refresh();
        }

        Box::new(backend)
    }

    pub fn current_style(&self) -> Rc<ObservedStyle> {
        self.current_style.borrow().clone()
    }

    pub fn stream(&self) -> Receiver<ObservedScreen> {
        self.screen_channel.1.clone()
    }

    pub fn input(&self) -> Sender<Option<Event>> {
        self.inner_sender.clone()
    }
}

impl backend::Backend for Backend {

    fn poll_event(&mut self) -> Option<Event> {
        match self.inner_receiver.try_recv() {
            Ok(event) => Some(Event),
            Err(TryRecvError::Empty) => None,
            Err(e) => panic!(e)
        }
    }

    fn finish(&mut self) {}

    fn refresh(&mut self) {
        let size = self.size.get_mut().clone();
        let current_frame =
            self.current_frame.replace(ObservedScreen::new(size));
        self.prev_frame.replace(Some(current_frame.clone()));
        self.screen_channel.0.send(current_frame).unwrap();
    }

    fn has_colors(&self) -> bool {
        true
    }

    fn screen_size(&self) -> Vec2 {
        self.size.borrow().clone()
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        let style = self.current_style.borrow().clone();
        let mut screen = self.current_frame.borrow_mut();
        let mut offset: usize = 0;

        for (idx, grapheme) in text.graphemes(true).enumerate() {
            let cpos = pos + Vec2::new(idx + offset, 0);
            screen[&cpos] = Some(ObservedCell::new(
                cpos,
                style.clone(),
                Some(grapheme.to_string()),
            ));

            for _ in 0..grapheme.width() - 1 {
                offset += 1;
                let spos = pos + Vec2::new(idx + offset, 0);
                screen[&spos] = Some(ObservedCell::new(spos, style.clone(), None));
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