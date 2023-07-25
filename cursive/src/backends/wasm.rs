#![cfg(feature = "wasm-backend")]

use cursive_core::{
    event::{ Event, Key },
    Vec2,
    theme,
};
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use web_sys::HtmlCanvasElement;
use wasm_bindgen::prelude::*;
use crate::backend;

#[wasm_bindgen]
#[derive(Debug, PartialEq)]
#[repr(C)]
struct TextColorPair  {
    text: char,
    color: ColorPair,
}

impl TextColorPair {
    pub fn new(text: char, color: ColorPair) -> Self {
        Self {
            text,
            color,
        }
    }
}

fn text_color_pairs_to_bytes(buffer: &Vec<TextColorPair>) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            buffer.as_ptr() as *const u8,
            buffer.len() * std::mem::size_of::<TextColorPair>(),
        )
    }
}

impl Clone for TextColorPair {
    fn clone(&self) -> Self {
        Self {
            text: self.text,
            color: self.color.clone(),
        }
    }
}


#[wasm_bindgen(module = "/src/backends/canvas.js")]
extern "C" {
    fn paint(buffer: &[u8]);
}

/// Backend using wasm.
pub struct Backend {
    canvas: HtmlCanvasElement,
    color: RefCell<ColorPair>,
    events: Rc<RefCell<VecDeque<Event>>>,
    buffer: RefCell<Vec<TextColorPair>>,
}
impl Backend {
    /// Creates a new Cursive root using a wasm backend.
    pub fn init() -> std::io::Result<Box<dyn backend::Backend>> {
        let document = web_sys::window()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get window",
            ))?
            .document()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get document",
            ))?;
        let canvas = document.get_element_by_id("cursive-wasm-canvas")
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get window",
            ))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to cast canvas",
            ))?;
        canvas.set_width(1000);
        canvas.set_height(1000);

        let color = cursive_to_color_pair(theme::ColorPair {
            front: theme::Color::Light(theme::BaseColor::Black),
            back:theme::Color::Dark(theme::BaseColor::Green),
        });

        let events = Rc::new(RefCell::new(VecDeque::new()));
         let cloned = events.clone();
         let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            match event.key_code() {
                8 => cloned.borrow_mut().push_back(Event::Key(Key::Backspace)),
                13 => cloned.borrow_mut().push_back(Event::Key(Key::Enter)),
                37 => cloned.borrow_mut().push_back(Event::Key(Key::Left)),
                38 => cloned.borrow_mut().push_back(Event::Key(Key::Up)),
                39 => cloned.borrow_mut().push_back(Event::Key(Key::Right)),
                40 => cloned.borrow_mut().push_back(Event::Key(Key::Down)),
                code => {
                    if let Some(c) = std::char::from_u32(code) {
                        cloned.borrow_mut().push_back(Event::Char(c));
                    }
                }            
            }
         }) as Box<dyn FnMut(_)>);
         document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to add event listener",
            ))?;
         closure.forget();

        let buffer = vec![TextColorPair::new(' ', color.clone()); 1_000_000];

        let c = Backend {
            canvas,
            color: RefCell::new(color),
            events,     
            buffer: RefCell::new(buffer),
         };
        Ok(Box::new(c))
    }
}

impl cursive_core::backend::Backend for Backend {
    fn poll_event(self: &mut Backend) -> Option<Event> {
        self.events.borrow_mut().pop_front()
    }

    fn set_title(self: &mut Backend, title: String) {
        self.canvas.set_title(&title);
    }

    fn refresh(self: &mut Backend) {
        let data = self.buffer.borrow().clone();
        paint(text_color_pairs_to_bytes(&data));
    }

    fn has_colors(self: &Backend) -> bool {
        true
    }

    fn screen_size(self: &Backend) -> Vec2 {
        Vec2::new(self.canvas.width() as usize, self.canvas.height() as usize)
    }

    fn print_at(self: &Backend, pos: Vec2, text: &str) {
        let color = (*self.color.borrow()).clone();
        let mut buffer = self.buffer.borrow_mut();
        for (i, c) in text.chars().enumerate() {
            let x = pos.x + i;
            buffer[1000 * pos.y + x] = TextColorPair::new(c, color.clone());
        }
    }

    fn clear(self: &Backend, _color: cursive_core::theme::Color) {
    }

    fn set_color(self: &Backend, color_pair: cursive_core::theme::ColorPair) -> cursive_core::theme::ColorPair {
        let mut color = self.color.borrow_mut();
        *color = cursive_to_color_pair(color_pair);
        color_pair
    }

    fn set_effect(self: &Backend, _: cursive_core::theme::Effect) {
    }

    fn unset_effect(self: &Backend, _: cursive_core::theme::Effect) {
    }

    fn name(&self) -> &str {
        "cursive-wasm-backend"
    }
}


/// Type of hex color which is r,g,b
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    red: u8, 
    green: u8,
    blue: u8
}

impl Color {
    /// Creates a new `Color` with the given red, green, and blue values.
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
        }
    }
}

/// Type of color pair.
#[derive(Clone, Debug, PartialEq, Eq)] 
pub struct ColorPair {
    /// Foreground text color.
    pub front: Color,
    /// Background color.
    pub back: Color,
}

/// Convert cursive color to hex color.
pub fn cursive_to_color(color: theme::Color) -> Color {
    match color {
        theme::Color::Dark(theme::BaseColor::Black) => Color::new(0,0,0),
        theme::Color::Dark(theme::BaseColor::Red) => Color::new(128,0,0),
        theme::Color::Dark(theme::BaseColor::Green) => Color::new(0,128,0),
        theme::Color::Dark(theme::BaseColor::Yellow) => Color::new(128,128,0),
        theme::Color::Dark(theme::BaseColor::Blue) => Color::new(0,0,128),
        theme::Color::Dark(theme::BaseColor::Magenta) => Color::new(128,0,128),
        theme::Color::Dark(theme::BaseColor::Cyan) => Color::new(0,128,128),
        theme::Color::Dark(theme::BaseColor::White) => Color::new(182,182,182),
        theme::Color::Light(theme::BaseColor::Black) => Color::new(128,128,128),
        theme::Color::Light(theme::BaseColor::Red) => Color::new(255,0,0),
        theme::Color::Light(theme::BaseColor::Green) => Color::new(0,0,255),
        theme::Color::Light(theme::BaseColor::Yellow) => Color::new(255,255,0),
        theme::Color::Light(theme::BaseColor::Blue) => Color::new(0,0,255),
        theme::Color::Light(theme::BaseColor::Magenta) => Color::new(255,0,255),
        theme::Color::Light(theme::BaseColor::Cyan) => Color::new(0,255,255),
        theme::Color::Light(theme::BaseColor::White) => Color::new(255,255,255),
        theme::Color::Rgb(r, g, b) =>  Color::new(r,g,b),
        theme::Color::RgbLowRes(r,g ,b ) =>  Color::new(r,g,b),
        theme::Color::TerminalDefault =>  Color::new(0,255,0),
    }
}

/// Convert cursive color pair to hex color pair.
pub fn cursive_to_color_pair(c: theme::ColorPair) -> ColorPair {
    ColorPair {
        front: cursive_to_color(c.front),
        back: cursive_to_color(c.back),
    }
}
