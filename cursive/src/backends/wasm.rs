#![cfg(feature = "wasm-backend")]

use wasm_bindgen::prelude::*;
use crate::backend;

const BUFFER_WIDTH: usize = 100;
const BUFFER_HEIGHT: usize = 100;

#[wasm_bindgen]
#[derive(Debug, PartialEq)]
#[repr(C)]
struct ScreenChar  {
    text: char,
    color: ColorPair,
}

impl ScreenChar {
    pub fn new(text: char, color: ColorPair) -> Self {
        Self {
            text,
            color,
        }
    }
}

impl Clone for ScreenChar {
    fn clone(&self) -> Self {
        Self {
            text: self.text,
            color: self.color.clone(),
        }
    }
}

#[repr(transparent)]
struct Buffer {
    chars: Vec<ScreenChar>,
}

impl Buffer {
    pub fn new(color: ColorPair) -> Self {
        Self {
            chars: vec![ScreenChar::new(' ', color.clone()); BUFFER_WIDTH * BUFFER_HEIGHT],
        }
    }

    pub fn set(self: &mut Buffer, index: usize, screen_char: ScreenChar) {
        self.chars[index] = screen_char;
    }
}

struct Writer {
    buffer: Buffer,
    color: ColorPair,
}

impl Writer {
    pub fn new() -> Self {
        let color = terminal_default_color_pair();
        Self {
            buffer: Buffer::new(color.clone()),
            color,
        }
    }

    pub fn write(self: &mut Writer, pos: Vec2, text: &str) {
        for (i, c) in text.chars().enumerate() {
            let x = pos.x + i;
            self.buffer.set(BUFFER_WIDTH * pos.y + x, ScreenChar::new(c, self.color.clone()));
        }
    }

    pub fn set_color(self: &mut Writer, color: ColorPair) {
        self.color = color;
    }

    pub fn clear(self: &mut Writer, color: Color) {
        let screen_char = ScreenChar::new(' ', ColorPair::new(color, color));

        self.buffer
            .chars
            .iter_mut()
            .for_each(|c| *c = screen_char.clone());
    }

    pub fn buffer(self: &Writer) -> &Vec<ScreenChar> {
        &self.buffer.chars
    }
}

/// Type of hex color which is r,g,b
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
struct Color {
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

use cursive_core::theme::{ Color as CColor, BaseColor as CBaseColor };

/// Convert cursive color to hex color.
fn cursive_to_color(color: CColor) -> Color {
    match color {
        CColor::Dark(CBaseColor::Black) => Color::new(0,0,0),
        CColor::Dark(CBaseColor::Red) => Color::new(128,0,0),
        CColor::Dark(CBaseColor::Green) => Color::new(0,128,0),
        CColor::Dark(CBaseColor::Yellow) => Color::new(128,128,0),
        CColor::Dark(CBaseColor::Blue) => Color::new(0,0,128),
        CColor::Dark(CBaseColor::Magenta) => Color::new(128,0,128),
        CColor::Dark(CBaseColor::Cyan) => Color::new(0,128,128),
        CColor::Dark(CBaseColor::White) => Color::new(182,182,182),
        CColor::Light(CBaseColor::Black) => Color::new(128,128,128),
        CColor::Light(CBaseColor::Red) => Color::new(255,0,0),
        CColor::Light(CBaseColor::Green) => Color::new(0,255, 0),
        CColor::Light(CBaseColor::Yellow) => Color::new(255,255,0),
        CColor::Light(CBaseColor::Blue) => Color::new(0,0,255),
        CColor::Light(CBaseColor::Magenta) => Color::new(255,0,255),
        CColor::Light(CBaseColor::Cyan) => Color::new(0,255,255),
        CColor::Light(CBaseColor::White) => Color::new(255,255,255),
        CColor::Rgb(r, g, b) =>  Color::new(r,g,b),
        CColor::RgbLowRes(r,g ,b ) =>  Color::new(r,g,b),
        CColor::TerminalDefault =>  Color::new(0,255,0),
    }
}

/// Type of color pair.
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
struct ColorPair {
    /// Foreground text color.
    pub front: Color,
    /// Background color.
    pub back: Color,
}

impl ColorPair {
    /// Creates a new `ColorPair` with the given foreground and background colors.
    pub fn new(front: Color, back: Color) -> Self {
        Self {
            front,
            back,
        }
    }
}

use cursive_core::theme::ColorPair as CColorPair;
/// Convert cursive color pair to hex color pair.
fn cursive_to_color_pair(c: CColorPair) -> ColorPair {
    ColorPair {
        front: cursive_to_color(c.front),
        back: cursive_to_color(c.back),
    }
}

fn terminal_default_color_pair() -> ColorPair {
    cursive_to_color_pair(CColorPair {
    front: CColor::Light(CBaseColor::Black),
    back:CColor::Dark(CBaseColor::Green),
    })
}

#[wasm_bindgen(module = "/src/backends/canvas.js")]
extern "C" {
    fn paint(buffer: &[u8]);
}

use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use web_sys::{ Document, HtmlCanvasElement };

/// Backend using wasm.
pub struct Backend {
    canvas: HtmlCanvasElement,
    events: Rc<RefCell<VecDeque<Event>>>,
    writer: RefCell<Writer>,
}

use cursive_core::{
    event::{ Event, Key },
    Vec2,
    theme::Effect,
};

impl Backend {
    /// Creates a new Cursive root using a wasm backend and given HTML canvas.
    pub fn new(canvas: HtmlCanvasElement) -> std::io::Result<Box<dyn backend::Backend>> {
        let document = Self::document()?;
        let events = Rc::new(RefCell::new(VecDeque::new()));
        let cloned = events.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let event = Self::to_cursive_event(event);
            if event != Event::Unknown(Vec::new()) {
                cloned.borrow_mut().push_back(event);
            }
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to add event listener",
            ))?;
        closure.forget();
        
        let c = Backend {
            canvas,
            events,
            writer: RefCell::new(Writer::new()),
        };
        Ok(Box::new(c))
    }

    /// Creates a new Cursive root using a wasm backend.
    pub fn init() -> std::io::Result<Box<dyn backend::Backend>> {
        let canvas = Self::canvas()?;
        canvas.set_width(1000);
        canvas.set_height(1000);

        Self::new(canvas)
    }

    fn document() -> Result<Document, std::io::Error> {
        web_sys::window()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get window",
            ))?
            .document()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get document",
            ))
    }

    fn canvas() -> Result<HtmlCanvasElement, std::io::Error> {
        Self::document()?
            .get_element_by_id("cursive-wasm-canvas")
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get window",
            ))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to cast canvas",
            ))
    }

    fn terminal_default_color() -> ColorPair {
        terminal_default_color_pair()
    }

    fn to_cursive_event(event: web_sys::KeyboardEvent) -> Event {
        match event.key_code() {
            8 => Event::Key(Key::Backspace),
            9 => Event::Key(Key::Tab),
            13 => Event::Key(Key::Enter),
            19 => Event::Key(Key::PauseBreak),
            27 => Event::Key(Key::Esc),
            33 => Event::Key(Key::PageUp),
            34 => Event::Key(Key::PageDown),
            35 => Event::Key(Key::End),
            36 => Event::Key(Key::Home),
            37 => Event::Key(Key::Left),
            38 => Event::Key(Key::Up),
            39 => Event::Key(Key::Right),
            40 => Event::Key(Key::Down),
            45 => Event::Key(Key::Ins),
            46 => Event::Key(Key::Del),
            101 => Event::Key(Key::NumpadCenter),
            112 => Event::Key(Key::F1),
            113 => Event::Key(Key::F2),
            114 => Event::Key(Key::F3),
            115 => Event::Key(Key::F4),
            116 => Event::Key(Key::F5),
            117 => Event::Key(Key::F6),
            118 => Event::Key(Key::F7),
            119 => Event::Key(Key::F8),
            120 => Event::Key(Key::F9),
            121 => Event::Key(Key::F10),
            122 => Event::Key(Key::F11),
            123 => Event::Key(Key::F12),
            code => {
                if let Some(c) = std::char::from_u32(code) {
                    Event::Char(c)
                } else { Event::Unknown(Vec::new()) }
            }  
        }
    }

    fn text_color_pairs_to_bytes(self: &Backend) -> &[u8] {
        let binding = self.writer.borrow();
        let data = binding.buffer();
        unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * std::mem::size_of::<ScreenChar>(),
            )
        }
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
        paint(self.text_color_pairs_to_bytes());
    }

    fn has_colors(self: &Backend) -> bool {
        true
    }

    fn screen_size(self: &Backend) -> Vec2 {
        Vec2::new(BUFFER_WIDTH, BUFFER_HEIGHT)
    }

    fn print_at(self: &Backend, pos: Vec2, text: &str) {
        self.writer.borrow_mut().write(pos, text);
    }

    fn clear(self: &Backend, color: CColor) {
        self.writer.borrow_mut().clear(cursive_to_color(color))
    }

    fn set_color(self: &Backend, color_pair: CColorPair) -> CColorPair {
        self.writer.borrow_mut().set_color(cursive_to_color_pair(color_pair));
        color_pair
    }

    fn set_effect(self: &Backend, _: Effect) {
    }

    fn unset_effect(self: &Backend, _: Effect) {
    }

    fn name(&self) -> &str {
        "cursive-wasm-backend"
    }
}
