extern crate ncurses;

/// Module for user-input events and their effects.
pub mod event;
/// Define various views to use when creating the layout.
pub mod view;
mod box_view;
mod stack_view;
mod text_view;

mod div;

use view::View;
use stack_view::StackView;

use event::EventResult;

/// Central part of the cursive library.
/// It initializes ncurses on creation and cleans up on drop.
/// To use it, you should populate it with views, layouts and callbacks,
/// then start the event loop with run().
pub struct Cursive {
    stacks: StackView,

    running: bool,
}

impl Cursive {
    /// Creates a new Cursive root, and initialize ncurses.
    pub fn new() -> Self {
        ncurses::initscr();
        ncurses::keypad(ncurses::stdscr, true);
        ncurses::noecho();

        Cursive{
            stacks: StackView::new(),
            running: true,
        }
    }

    /// Runs the event loop.
    /// It will wait for user input (key presses) and trigger callbacks accordingly.
    /// Blocks until quit() is called.
    pub fn run(&mut self) {
        while self.running {
            ncurses::refresh();

            // Handle event
            let ch = ncurses::getch();
            match self.stacks.on_key_event(ch) {
                EventResult::Ignored => (),
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

/// Simple 2D size, in characters.
#[derive(Clone,Copy)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

impl Size {
    pub fn new(w: u32, h: u32) -> Self {
        Size {
            w: w,
            h: h,
        }
    }
}

/// A generic trait for converting a value into a 2D size
pub trait ToSize {
    fn to_size(self) -> Size;
}

impl ToSize for Size {
    fn to_size(self) -> Size {
        self
    }
}

impl ToSize for (u32,u32) {
    fn to_size(self) -> Size {
        Size::new(self.0, self.1)
    }
}
