//! # Cursive
//!
//! Cursive is a TUI library built on top of ncurses-rs.
//! It allows to easily build layouts for text-based applications.
//!
//! ## Example
//! ```
//! extern crate cursive;
//!
//! use cursive::Cursive;
//! use cursive::view::TextView;
//!
//! fn main() {
//!     let mut siv = Cursive::new();
//!
//!     siv.add_layer(TextView::new("Hello World!\nPress q to quit."));
//!
//!     siv.add_global_callback('q' as i32, |s| s.quit());
//!
//!     siv.run();
//! }
//! ```

extern crate ncurses;

pub mod event;
pub mod view;
pub mod printer;
pub mod vec2;
mod box_view;
mod stack_view;
mod text_view;

mod div;

use std::rc::Rc;
use std::collections::HashMap;

use view::View;
use stack_view::StackView;

use event::{EventResult,Callback};

/// Identifies a screen in the cursive ROOT.
pub type ScreenId = usize;

/// Central part of the cursive library.
/// It initializes ncurses on creation and cleans up on drop.
/// To use it, you should populate it with views, layouts and callbacks,
/// then start the event loop with run().
///
/// It uses a list of screen, with one screen active at a time.
pub struct Cursive {
    screens: Vec<StackView>,

    active_screen: ScreenId,

    running: bool,

    global_callbacks: HashMap<i32, Rc<Callback>>,
}

impl Cursive {
    /// Creates a new Cursive root, and initialize ncurses.
    pub fn new() -> Self {
        ncurses::initscr();
        ncurses::keypad(ncurses::stdscr, true);
        ncurses::noecho();
        ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

        let mut res = Cursive {
            screens: Vec::new(),
            active_screen: 0,
            running: true,
            global_callbacks: HashMap::new(),
        };

        res.screens.push(StackView::new());

        res
    }

    /// Returns a mutable reference to the currently active screen.
    pub fn screen_mut(&mut self) -> &mut StackView {
        let id = self.active_screen;
        self.screens.get_mut(id).unwrap()
    }

    /// Adds a new screen, and returns its ID.
    pub fn add_screen(&mut self) -> ScreenId {
        let res = self.screens.len();
        self.screens.push(StackView::new());
        res
    }

    /// Convenient method to create a new screen, and set it as active.
    pub fn add_active_screen(&mut self) -> ScreenId {
        let res = self.add_screen();
        self.set_screen(res);
        res
    }

    /// Sets the active screen. Panics if no such screen exist.
    pub fn set_screen(&mut self, screen_id: ScreenId) {
        if screen_id >= self.screens.len() {
            panic!("Tried to set an invalid screen ID: {}, but only {} screens present.", screen_id, self.screens.len());
        }
        self.active_screen = screen_id;
    }

    /// Adds a global callback, triggered on the given key press when no view catches it.
    pub fn add_global_callback<F>(&mut self, key: i32, cb: F)
        where F: Fn(&mut Cursive) + 'static
    {
        self.global_callbacks.insert(key, Rc::new(Box::new(cb)));
    }

    /// Convenient method to add a layer to the current screen.
    pub fn add_layer<T: 'static + View>(&mut self, view: T) {
        self.screen_mut().add_layer(view);
    }

    fn on_key_event(&mut self, ch: i32) {
        let cb = match self.global_callbacks.get(&ch) {
            None => return,
            Some(cb) => cb.clone(),
        };
        cb(self);
    }

    /// Runs the event loop.
    /// It will wait for user input (key presses) and trigger callbacks accordingly.
    /// Blocks until quit() is called.
    pub fn run(&mut self) {
        while self.running {
            ncurses::refresh();

            // Handle event
            let ch = ncurses::getch();

            match self.screen_mut().on_key_event(ch) {
                EventResult::Ignored => self.on_key_event(ch),
                EventResult::Consumed(None) => (),
                EventResult::Consumed(Some(cb)) => cb(self),
            }
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
        println!("Quitting now!");
    }
}

impl Drop for Cursive {
    fn drop(&mut self) {
        ncurses::endwin();
    }
}

