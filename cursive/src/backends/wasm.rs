#![cfg(feature = "wasm-backend")]

use cursive_core::{
    event::Event,
    Vec2,
    theme,
};
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use web_sys::{
    HtmlCanvasElement,
    CanvasRenderingContext2d,
};
use wasm_bindgen::prelude::*;
use crate::backend;


/// Backend using wasm.
pub struct Backend {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    color: RefCell<ColorPair>,
    font_height: usize,
    font_width: usize,
    events: Rc<RefCell<VecDeque<Event>>>,
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

        let font_width = 12;     
        let font_height = font_width * 2;
        let ctx: CanvasRenderingContext2d = canvas.get_context("2d")
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get canvas context",
            ))?
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get canvas context",
            ))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to cast canvas context",
            ))?;
        ctx.set_font(&format!("{}px monospace", font_height));

        let color = RefCell::new(cursive_to_color_pair(theme::ColorPair {
            front: theme::Color::Light(theme::BaseColor::Black),
            back:theme::Color::Dark(theme::BaseColor::Green),
        }));

        let events = Rc::new(RefCell::new(VecDeque::new()));
         let cloned = events.clone();
         let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
             for c in event.key().chars() {
                cloned.borrow_mut().push_back(Event::Char(c));
             }
         }) as Box<dyn FnMut(_)>);
         canvas.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to add event listener",
            ))?;
         closure.forget();

         let c = Backend { 
            canvas,
            ctx,
            color,
            font_height,
            font_width,
            events,     
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

    fn refresh(self: &mut Backend) {}

    fn has_colors(self: &Backend) -> bool {
        true
    }

    fn screen_size(self: &Backend) -> Vec2 {
        Vec2::new(self.canvas.width() as usize, self.canvas.height() as usize)
    }

    fn print_at(self: &Backend, pos: Vec2, text: &str) {
        let color = self.color.borrow();
        self.ctx.set_fill_style(&JsValue::from_str(&color.back));
        self.ctx.fill_rect((pos.x * self.font_width) as f64, (pos.y * self.font_height) as f64, (self.font_width * text.len()) as f64, self.font_height as f64);
        self.ctx.set_fill_style(&JsValue::from_str(&color.front));
        self.ctx.fill_text(text, (pos.x * self.font_width) as f64, (pos.y * self.font_height + self.font_height * 3/4) as f64).unwrap();
    }

    fn clear(self: &Backend, color: cursive_core::theme::Color) {
        self.ctx.set_fill_style(&JsValue::from_str(&cursive_to_color(color)));
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


/// Type of hex color which starts with #.
pub type Color = String;

/// Type of color pair. 
pub struct ColorPair {
    /// Foreground text color.
    pub front: Color,
    /// Background color.
    pub back: Color,
}

/// Convert cursive color to hex color.
pub fn cursive_to_color(color: theme::Color) -> Color {
    match color {
        theme::Color::Dark(theme::BaseColor::Black) => "#000000".to_string(),
        theme::Color::Dark(theme::BaseColor::Red) => "#800000".to_string(),
        theme::Color::Dark(theme::BaseColor::Green) => "#008000".to_string(),
        theme::Color::Dark(theme::BaseColor::Yellow) => "#808000".to_string(),
        theme::Color::Dark(theme::BaseColor::Blue) => "#000080".to_string(),
        theme::Color::Dark(theme::BaseColor::Magenta) => "#800080".to_string(),
        theme::Color::Dark(theme::BaseColor::Cyan) => "#008080".to_string(),
        theme::Color::Dark(theme::BaseColor::White) => "#c0c0c0".to_string(),
        theme::Color::Light(theme::BaseColor::Black) => "#808080".to_string(),
        theme::Color::Light(theme::BaseColor::Red) => "#ff0000".to_string(),
        theme::Color::Light(theme::BaseColor::Green) => "#00ff00".to_string(),
        theme::Color::Light(theme::BaseColor::Yellow) => "#ffff00".to_string(),
        theme::Color::Light(theme::BaseColor::Blue) => "#0000ff".to_string(),
        theme::Color::Light(theme::BaseColor::Magenta) => "#ff00ff".to_string(),
        theme::Color::Light(theme::BaseColor::Cyan) => "#00ffff".to_string(),
        theme::Color::Light(theme::BaseColor::White) => "#ffffff".to_string(),
        theme::Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b).to_string(),
        theme::Color::RgbLowRes(r,g ,b ) => format!("#{:01x}{:01x}{:01x}", r, g, b).to_string(),
        theme::Color::TerminalDefault => "#00ff00".to_string(),
    }
}

/// Convert cursive color pair to hex color pair.
pub fn cursive_to_color_pair(c: theme::ColorPair) -> ColorPair {
    ColorPair {
        front: cursive_to_color(c.front),
        back: cursive_to_color(c.back),
    }
}
