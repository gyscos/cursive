//! # Cursive
//!
//! Cursive is a TUI library built on top of ncurses-rs.
//! It allows to easily build layouts for text-based applications.
//!
//! ## Example
//! ```no_run
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
//!     siv.add_global_callback('q', |s| s.quit());
//!
//!     siv.run();
//! }
//! ```
extern crate ncurses;
extern crate toml;

pub mod event;
pub mod view;
pub mod printer;
pub mod vec;
pub mod theme;
pub mod align;
pub mod orientation;

mod div;
mod utf8;

use std::any::Any;
use std::rc::Rc;
use std::collections::HashMap;
use std::path::Path;

use vec::Vec2;
use printer::Printer;
use view::View;
use view::{StackView, Selector};

use event::{Event, ToEvent, Key, EventResult, Callback};

/// Identifies a screen in the cursive ROOT.
pub type ScreenId = usize;

/// Central part of the cursive library.
///
/// It initializes ncurses on creation and cleans up on drop.
/// To use it, you should populate it with views, layouts and callbacks,
/// then start the event loop with run().
///
/// It uses a list of screen, with one screen active at a time.
pub struct Cursive {
    screens: Vec<StackView>,

    active_screen: ScreenId,

    running: bool,

    global_callbacks: HashMap<Event, Rc<Callback>>,

    theme: theme::Theme,
}

impl Cursive {
    /// Creates a new Cursive root, and initialize ncurses.
    pub fn new() -> Self {
        // Default delay is way too long. 25 is imperceptible yet works fine.
        std::env::set_var("ESCDELAY", "25");
        ncurses::setlocale(ncurses::LcCategory::all, "");
        ncurses::initscr();
        ncurses::keypad(ncurses::stdscr, true);
        ncurses::noecho();
        ncurses::cbreak();
        ncurses::start_color();
        ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        let theme = theme::load_default();
        // let theme = theme::load_theme("assets/style.toml").unwrap();

        ncurses::wbkgd(ncurses::stdscr,
                       ncurses::COLOR_PAIR(theme::ColorPair::Background.ncurses_id()));

        let mut res = Cursive {
            screens: Vec::new(),
            active_screen: 0,
            running: true,
            global_callbacks: HashMap::new(),
            theme: theme,
        };

        res.screens.push(StackView::new());

        res
    }

    /// Returns the currently used theme
    pub fn current_theme(&self) -> &theme::Theme {
        &self.theme
    }

    /// Loads a theme from the given file.
    ///
    /// Returns TRUE if the theme was successfully loaded.
    pub fn load_theme<P: AsRef<Path>>(&mut self, filename: P) -> bool {
        match theme::load_theme(filename) {
            Err(_) => return false,
            Ok(theme) => self.theme = theme,
        }
        true
    }

    /// Regularly redraws everything, even when no input is given. Between 0 and 1000.
    ///
    /// Call with fps=0 to disable (default value).
    pub fn set_fps(&self, fps: u32) {
        if fps == 0 {
            ncurses::timeout(-1);
        } else {
            ncurses::timeout(1000 / fps as i32);
        }
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
            panic!("Tried to set an invalid screen ID: {}, but only {} screens present.",
                   screen_id,
                   self.screens.len());
        }
        self.active_screen = screen_id;
    }

    fn find_any(&mut self, selector: &Selector) -> Option<&mut Any> {
        // Internal find method that returns a Any object.
        self.screen_mut().find(selector)
    }

    /// Tries to find the view pointed to by the given path.
    /// If the view is not found, or if it is not of the asked type,
    /// it returns None.
    pub fn find<V: View + Any>(&mut self, selector: &Selector) -> Option<&mut V> {
        match self.find_any(selector) {
            None => None,
            Some(b) => b.downcast_mut::<V>(),
        }
    }

    /// Convenient method to use `find` with a `Selector::Id`.
    pub fn find_id<V: View + Any>(&mut self, id: &str) -> Option<&mut V> {
        self.find(&Selector::Id(id))
    }

    /// Adds a global callback, triggered on the given key press when no view catches it.
    pub fn add_global_callback<F, E: ToEvent>(&mut self, event: E, cb: F)
        where F: Fn(&mut Cursive) + 'static
    {
        self.global_callbacks.insert(event.to_event(), Rc::new(Box::new(cb)));
    }

    /// Convenient method to add a layer to the current screen.
    pub fn add_layer<T: 'static + View>(&mut self, view: T) {
        self.screen_mut().add_layer(view);
    }

    /// Convenient method to remove a layer from the current screen.
    pub fn pop_layer(&mut self) {
        self.screen_mut().pop_layer();
    }

    // Handles a key event when it was ignored by the current view
    fn on_event(&mut self, event: Event) {
        let cb = match self.global_callbacks.get(&event) {
            None => return,
            Some(cb) => cb.clone(),
        };
        // Not from a view, so no viewpath here
        cb(self);
    }

    /// Returns the size of the screen, in characters.
    pub fn screen_size(&self) -> Vec2 {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        ncurses::getmaxyx(ncurses::stdscr, &mut y, &mut x);

        Vec2 {
            x: x as usize,
            y: y as usize,
        }
    }

    fn layout(&mut self) {
        let size = self.screen_size();
        self.screen_mut().layout(size);
    }

    fn draw(&mut self) {
        let printer = Printer::new(self.screen_size(), self.theme.clone());
        self.screen_mut().draw(&printer);
        ncurses::refresh();
    }

    fn poll_event() -> Event {
        let ch: i32 = ncurses::getch();

        // Is it a UTF-8 starting point?
        if 32 <= ch && ch < 0x100 && ch != 127 {
            Event::CharEvent(utf8::read_char(ch as u8, || ncurses::getch() as u8).unwrap())
        } else {
            Event::KeyEvent(Key::from_ncurses(ch))
        }
    }

    /// Runs the event loop.
    /// It will wait for user input (key presses) and trigger callbacks accordingly.
    /// Blocks until quit() is called.
    pub fn run(&mut self) {

        // And the big event loop begins!
        while self.running {
            // Do we need to redraw everytime?
            // Probably, actually.
            // TODO: Do we actually need to clear everytime?
            ncurses::clear();
            // TODO: Do we need to re-layout everytime?
            self.layout();
            // TODO: Do we need to redraw every view every time?
            // (Is this getting repetitive? :p)
            self.draw();

            // Wait for next event.
            // (If set_fps was called, this returns -1 now and then)
            let event = Cursive::poll_event();

            match self.screen_mut().on_event(event) {
                // If the event was ignored, it is our turn to play with it.
                EventResult::Ignored => self.on_event(event),
                EventResult::Consumed(None) => (),
                EventResult::Consumed(Some(cb)) => cb(self),
            }
        }
    }

    /// Stops the event loop.
    pub fn quit(&mut self) {
        self.running = false;
    }
}

impl Drop for Cursive {
    fn drop(&mut self) {
        ncurses::endwin();
    }
}
