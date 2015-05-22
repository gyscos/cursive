use std::cmp::max;

use ncurses;

use color;
use vec::Vec2;
use view::{View,SizeRequest,DimensionRequest};
use event::EventResult;
use printer::Printer;

/// Simple stack of views.
/// Only the top-most view is active and can receive input.
pub struct StackView {
    layers: Vec<Layer>,
}

struct Layer {
    view: Box<View>,
    size: Vec2,
    win: Option<ncurses::WINDOW>,
}

impl StackView {
    /// Creates a new empty StackView
    pub fn new() -> Self {
        StackView {
            layers: Vec::new(),
        }
    }

    /// Add new view on top of the stack.
    pub fn add_layer<T: 'static + View>(&mut self, view: T) {
        self.layers.push(Layer {
            view: Box::new(view),
            size: Vec2::new(0,0),
            win: None,
        });
    }

    /// Remove the top-most layer.
    pub fn pop_layer(&mut self) {
        self.layers.pop();
    }
}


impl View for StackView {
    fn draw(&self, printer: &Printer, focused: bool) {
        ncurses::wrefresh(printer.win);
        for v in self.layers.iter() {
            // Center the view
            v.view.draw(&Printer::new(v.win.unwrap(), v.size), focused);

            let h = v.size.y;
            let w = v.size.x;
            let x = (printer.size.x - w) / 2;
            let y = (printer.size.y - h) / 2;


            let printer = printer.style(color::SHADOW);
            printer.print_hline((x+1,y+h), w, ' ' as u64);
            printer.print_vline((x+w,y+1), h, ' ' as u64);

            // v.view.draw(&printer.sub_printer(offset, v.size), focused);
            ncurses::wrefresh(v.win.unwrap());
        }
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        match self.layers.last_mut() {
            None => EventResult::Ignored,
            Some(v) => v.view.on_key_event(ch),
        }
    }

    fn layout(&mut self, size: Vec2) {
        let req = SizeRequest {
            w: DimensionRequest::AtMost(size.x),
            h: DimensionRequest::AtMost(size.y),
        };
        for layer in self.layers.iter_mut() {
            layer.size = Vec2::min(size, layer.view.get_min_size(req));
            layer.view.layout(layer.size);

            let h = layer.size.y as i32;
            let w = layer.size.x as i32;
            let x = (size.x as i32 - w) / 2;
            let y = (size.y as i32 - h) / 2;
            let win = ncurses::newwin(h, w, y, x);
            ncurses::wbkgd(win, ncurses::COLOR_PAIR(color::PRIMARY));

            match layer.win {
                None => (),
                Some(w) => { ncurses::delwin(w); },
            }
            layer.win = Some(win);
        }
    }

    fn get_min_size(&self, size: SizeRequest) -> Vec2 {
        // The min size is the max of all children's
        let mut s = Vec2::new(1,1);

        for layer in self.layers.iter() {
            let vs = layer.view.get_min_size(size);
            s = Vec2::max(s, vs);
        }

        s
    }

    fn take_focus(&mut self) -> bool {
        match self.layers.last_mut() {
            None => false,
            Some(mut v) => v.view.take_focus()
        }
    }
}
