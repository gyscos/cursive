use std::rc::Rc;

use unicode_width::UnicodeWidthStr;

use menu::{MenuItem, MenuTree};
use printer::Printer;
use view::View;
use view::scroll::ScrollBase;
use align::Align;
use vec::Vec2;

/// fd
pub struct MenuPopup {
    menu: Rc<MenuTree>,
    focus: usize,
    scrollbase: ScrollBase,
    align: Align,
}

impl MenuPopup {
    pub fn new(menu: Rc<MenuTree>) -> Self {
        MenuPopup {
            menu: menu,
            focus: 0,
            scrollbase: ScrollBase::new(),
            align: Align::top_left(),
        }
    }

    /// Sets the alignment for this view.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;

        self
    }
}

impl View for MenuPopup {
    fn draw(&mut self, printer: &Printer) {
        let h = self.menu.len();
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer = &printer.sub_printer(Vec2::new(0, offset),
                                           printer.size,
                                           true);

        // Start with a box
        printer.print_box(Vec2::new(0, 0), printer.size);

        // We're giving it a reduced size because of borders.
        // But we're keeping the full width,
        // to integrate horizontal delimiters in the frame.
        let size = printer.size - (0, 2);
        let printer = printer.sub_printer(Vec2::new(0, 1), size, true);
        self.scrollbase.draw(&printer, |printer, i| {
            printer.with_selection(i == self.focus, |printer| {
                let item = &self.menu.children[i];
                match *item {
                    MenuItem::Delimiter => {
                        printer.print_hdelim((0, 0), printer.size.x)
                    }
                    MenuItem::Subtree(ref label, _) |
                    MenuItem::Leaf(ref label, _) => {
                        printer.print((2, 0), label)
                    }
                }

            });
        });
    }

    fn get_min_size(&self, req: Vec2) -> Vec2 {
        // We can't really shrink our items here, so it's not flexible.
        let w = self.menu
                    .children
                    .iter()
                    .map(|item| item.label().width())
                    .max()
                    .unwrap_or(1);
        let h = self.menu.children.len();


        let scrolling = req.y < h;

        let w = if scrolling {
            w + 2
        } else {
            w
        };

        Vec2::new(w, h)
    }
}
