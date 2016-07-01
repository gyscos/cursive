use menu::*;
use theme::ColorStyle;
use printer::Printer;
use event::*;

use std::rc::Rc;

pub struct Menubar {
    pub menu: MenuTree,
    pub autohide: bool,
    pub selected: bool,
}

impl Menubar {
    pub fn new() -> Self {
        Menubar {
            menu: MenuTree::new(),
            autohide: true,
            selected: false,
        }
    }

    pub fn draw(&mut self, printer: &Printer) {
        // Draw the bar at the top
        printer.with_color(ColorStyle::Primary, |printer| {
            printer.print_hline((0, 0), printer.size.x, " ");
        });

        // TODO: draw the rest
    }

    pub fn on_event(&mut self, event: Event) -> Option<Rc<Callback>> {
        let _ = &event;
        None
    }
}
