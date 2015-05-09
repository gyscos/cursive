extern crate ncurses;

pub mod focus;
pub mod view;

pub use self::view::{View,TextView,Button,Dialog,BackgroundView};

use std::ops::DerefMut;

pub struct Cursive {
    background: Box<View>,
    layers: Vec<Box<View>>,

    running: bool,
}

pub type Callback = Fn(&mut Cursive);

impl Cursive {
    pub fn new() -> Self {
        ncurses::initscr();
        ncurses::keypad(ncurses::stdscr, true);
        ncurses::noecho();

        Cursive{
            background: Box::new(BackgroundView),
            layers: Vec::new(),
            running: true,
        }
    }

    pub fn new_layer<V: 'static + View>(&mut self, view: V) {
        self.layers.push(Box::new(view));
    }

    pub fn run(&mut self) {
        while self.running {
            ncurses::refresh();

            // Handle event
            match ncurses::getch() {
                10 => {
                    let cb = self.layers.last_mut().unwrap_or(&mut self.background).click();
                    cb.map(|cb| cb(self));
                },
                ncurses::KEY_LEFT => { self.layers.last_mut().unwrap_or(&mut self.background).focus_left(); },
                ncurses::KEY_RIGHT => { self.layers.last_mut().unwrap_or(&mut self.background).focus_right(); },
                ncurses::KEY_DOWN => { self.layers.last_mut().unwrap_or(&mut self.background).focus_bottom(); },
                ncurses::KEY_UP => { self.layers.last_mut().unwrap_or(&mut self.background).focus_top(); },
                a => println!("Key: {}", a),
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
