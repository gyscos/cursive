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
// #![deny(missing_docs)]

extern crate ncurses;
extern crate toml;
extern crate unicode_segmentation;
extern crate unicode_width;

macro_rules! println_stderr(
    ($($arg:tt)*) => { {
        use ::std::io::Write;
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);


pub mod event;
pub mod view;
pub mod printer;
pub mod vec;
pub mod theme;
pub mod align;
pub mod orientation;
pub mod menu;

// This probably doesn't need to be public?
mod menubar;

mod div;
mod utf8;

mod backend;

use backend::{Backend, NcursesBackend};

use std::any::Any;
use std::rc::Rc;
use std::collections::HashMap;
use std::path::Path;

use vec::Vec2;
use printer::Printer;
use view::View;
use view::{Selector, StackView};

use event::{Callback, Event, EventResult, ToEvent};

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
    theme: theme::Theme,
    screens: Vec<StackView>,
    global_callbacks: HashMap<Event, Callback>,
    menubar: menubar::Menubar,

    active_screen: ScreenId,

    running: bool,
}

impl Default for Cursive {
    fn default() -> Self {
        Self::new()
    }
}

// Use the Ncurses backend.
// TODO: make this feature-driven
type B = NcursesBackend;

impl Cursive {
    /// Creates a new Cursive root, and initialize ncurses.
    pub fn new() -> Self {
        // Default delay is way too long. 25 is imperceptible yet works fine.
        B::init();

        let theme = theme::load_default();
        // let theme = theme::load_theme("assets/style.toml").unwrap();

        let mut res = Cursive {
            theme: theme,
            screens: Vec::new(),
            global_callbacks: HashMap::new(),
            menubar: menubar::Menubar::new(),
            active_screen: 0,
            running: true,
        };

        res.screens.push(StackView::new());

        res
    }

    /// Selects the menubar
    pub fn select_menubar(&mut self) {
        self.menubar.take_focus();
    }

    /// Sets the menubar autohide_menubar feature.
    ///
    /// * When enabled, the menu is only visible when selected.
    /// * When disabled, the menu is always visible and reserves the top row.
    pub fn set_autohide_menu(&mut self, autohide: bool) {
        self.menubar.autohide = autohide;
    }

    /// Retrieve the menu tree used by the menubar.
    pub fn menubar(&mut self) -> &mut menubar::Menubar {
        &mut self.menubar
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

    /// Sets the refresh rate, in frames per second.
    ///
    /// Regularly redraws everything, even when no input is given.
    /// Between 0 and 1000.
    /// Call with fps=0 to disable (default value).
    pub fn set_fps(&self, fps: u32) {
        B::set_refresh_rate(fps)
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
            panic!("Tried to set an invalid screen ID: {}, but only {} \
                    screens present.",
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
    pub fn find<V: View + Any>(&mut self, sel: &Selector) -> Option<&mut V> {
        match self.find_any(sel) {
            None => None,
            Some(b) => b.downcast_mut::<V>(),
        }
    }

    /// Convenient method to use `find` with a `Selector::Id`.
    pub fn find_id<V: View + Any>(&mut self, id: &str) -> Option<&mut V> {
        self.find(&Selector::Id(id))
    }

    /// Adds a global callback.
    ///
    /// Will be triggered on the given key press when no view catches it.
    pub fn add_global_callback<F, E: ToEvent>(&mut self, event: E, cb: F)
        where F: Fn(&mut Cursive) + 'static
    {
        self.global_callbacks.insert(event.to_event(), Rc::new(cb));
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
        let (x, y) = B::screen_size();

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
        // TODO: don't clone the theme
        // Reference it or something
        let printer = Printer::new(self.screen_size(), self.theme.clone());

        // Draw the currently active screen
        // If the menubar is active, nothing else can be.
        let offset = if self.menubar.autohide {
            0
        } else {
            1
        };
        // Draw the menubar?
        if self.menubar.visible() {
            let printer = printer.sub_printer(Vec2::zero(),
                                              printer.size,
                                              self.menubar.receive_events());
            self.menubar.draw(&printer);
        }

        let selected = self.menubar.receive_events();

        let printer =
            printer.sub_printer(Vec2::new(0, offset), printer.size, !selected);
        self.screen_mut().draw(&printer);

        B::refresh();
    }

    /// Runs the event loop.
    ///
    /// It will wait for user input (key presses)
    /// and trigger callbacks accordingly.
    ///
    /// Blocks until quit() is called.
    pub fn run(&mut self) {

        // And the big event loop begins!
        while self.running {
            // Do we need to redraw everytime?
            // Probably, actually.
            // TODO: Do we need to re-layout everytime?
            self.layout();

            // TODO: Do we need to redraw every view every time?
            // (Is this getting repetitive? :p)
            self.draw();

            // Wait for next event.
            // (If set_fps was called, this returns -1 now and then)
            let event = B::poll_event();
            if event == Event::Key(event::Key::Resize) {
                B::clear();
                continue;
            }

            // Event dispatch order:
            // * Focused element:
            //     * Menubar (if active)
            //     * Current screen (top layer)
            // * Global callbacks
            if self.menubar.receive_events() {
                if let Some(cb) = self.menubar.on_event(event) {
                    cb(self);
                }
            } else {
                match self.screen_mut().on_event(event) {
                    // If the event was ignored,
                    // it is our turn to play with it.
                    EventResult::Ignored => self.on_event(event),
                    EventResult::Consumed(None) => (),
                    EventResult::Consumed(Some(cb)) => cb(self),
                }
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
        B::finish();
    }
}
