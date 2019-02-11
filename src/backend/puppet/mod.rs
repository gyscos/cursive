#![warn(missing_docs)]
#![allow(warnings)]
#![warn(unused)]
#![allow(bad_style)]

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
pub mod observed_screen_view;

pub const DEFAULT_SIZE: Vec2 = XY::<usize> { x: 120, y: 80 };
pub const DEFAULT_OBSERVED_STYLE: ObservedStyle = ObservedStyle {
    colors: ColorPair {
        front: Color::TerminalDefault,
        back: Color::TerminalDefault,
    },
    effects: enum_set!(),
};

pub struct Backend {
    inner_sender: Sender<Option<Event>>,
    inner_receiver: Receiver<Option<Event>>,
    prev_frame: RefCell<Option<ObservedScreen>>,
    current_frame: RefCell<ObservedScreen>,
    size: RefCell<Vec2>,
    current_style: RefCell<Rc<ObservedStyle>>,
    screen_channel : (Sender<ObservedScreen>, Receiver<ObservedScreen>)
}

impl Backend {
    pub fn init(size_op : Option<Vec2>) -> Box<Backend>
    where
        Self: Sized,
    {
        let (inner_sender, inner_receiver) = crossbeam_channel::bounded(1);
        let size = size_op.unwrap_or(DEFAULT_SIZE);

        let mut backend = Backend {
            inner_sender,
            inner_receiver,
            prev_frame: RefCell::new(None),
            current_frame: RefCell::new(ObservedScreen::new(size)),
            size: RefCell::new(DEFAULT_SIZE),
            current_style: RefCell::new(Rc::new(DEFAULT_OBSERVED_STYLE)),
            screen_channel : crossbeam_channel::unbounded()
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
    fn finish(&mut self) {}

    fn start_input_thread(
        &mut self, event_sink: Sender<Option<Event>>,
        input_requests: Receiver<backend::InputRequest>,
    ) {
        let receiver = self.inner_receiver.clone();

        thread::spawn(move || {
            for _ in input_requests {
                match receiver.recv() {
                    Err(e) => {
                        println!("e1 {:?}", e);
                        return
                    },
                    Ok(event) => {
                        let res = event_sink.send(event);
                        if res.is_err() {
                            println!("e2 {:?}", res);
                            return;
                        } else {
                            println!("got event {:?}", res);
                        }
                    }
                }
            }
        });
    }

    fn refresh(&mut self) {
        let size = self.size.get_mut().clone();
        let current_frame = self.current_frame.replace(ObservedScreen::new(size));
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
        let mut skip: usize = 0;
        //since some graphemes are visually longer than one char, we need to track printer offset.
        let mut offset: usize = 0;

        let style = self.current_style.borrow().clone();
        let mut screen = self.current_frame.borrow_mut();

        let mut graphemes = text.graphemes(true);

        let mut idx = 0;
        'printer: while let Some(g) = graphemes.next() {
            let lpos = Vec2::new(pos.x + idx + offset, pos.y);
            idx += g.len();
            let charp : String = g.to_owned();
            // skipping the "continuation" tails
//            while skip > 0 {
//                screen[&pos] = Some(ObservedCell::new(style.clone(), None));
//
//                skip -= 1;
//                offset += 1;
//                continue 'printer;
//            }

            // if we got here, we have to write a new character.
            // TODO(njskalski): add the support for "multiple cell" characters.
            screen[&lpos] = Some(ObservedCell::new(lpos,style.clone(), Some(charp)));
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
