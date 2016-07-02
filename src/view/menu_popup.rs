use std::rc::Rc;

use unicode_width::UnicodeWidthStr;

use Cursive;
use menu::{MenuItem, MenuTree};
use printer::Printer;
use view::View;
use view::Position;
use view::scroll::ScrollBase;
use align::Align;
use vec::Vec2;
use event::{Callback, Event, EventResult, Key};

/// fd
pub struct MenuPopup {
    menu: Rc<MenuTree>,
    focus: usize,
    scrollbase: ScrollBase,
    align: Align,
    on_dismiss: Option<Callback>,
    on_action: Option<Callback>,
}

impl MenuPopup {
    pub fn new(menu: Rc<MenuTree>) -> Self {
        MenuPopup {
            menu: menu,
            focus: 0,
            scrollbase: ScrollBase::new(),
            align: Align::top_left(),
            on_dismiss: None,
            on_action: None,
        }
    }

    /// Sets the alignment for this view.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;

        self
    }

    pub fn on_dismiss<F: 'static + Fn(&mut Cursive)>(mut self, f: F) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }

    pub fn on_action<F: 'static + Fn(&mut Cursive)>(mut self, f: F) -> Self {
        self.on_action = Some(Rc::new(f));
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
                        printer.print_hline((1, 0), printer.size.x - 2, " ");
                        printer.print((2, 0), label);
                    }
                }

            });
        });
    }

    fn get_min_size(&self, req: Vec2) -> Vec2 {
        // We can't really shrink our items here, so it's not flexible.
        let w = 2 +
                self.menu
                    .children
                    .iter()
                    .map(|item| 2 + item.label().width())
                    .max()
                    .unwrap_or(1);
        let h = 2 + self.menu.children.len();


        let scrolling = req.y < h;

        let w = if scrolling {
            w + 2
        } else {
            w
        };

        Vec2::new(w, h)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Esc) => {
                let dismiss_cb = self.on_dismiss.clone();
                return EventResult::with_cb(move |s| {
                    if let Some(ref cb) = dismiss_cb {
                        cb.clone()(s);
                    }
                    s.pop_layer();
                });
            }
            Event::Key(Key::Up) if self.focus > 0 => self.focus -= 1,
            Event::Key(Key::Down) if self.focus + 1 <
                                     self.menu.children.len() => {
                self.focus += 1
            }
            Event::Key(Key::Enter) if !self.menu.children[self.focus]
                                          .is_delimiter() => {
                return match self.menu.children[self.focus] {
                    MenuItem::Leaf(_, ref cb) => {

                        let cb = cb.clone();
                        let action_cb = self.on_action.clone();
                        EventResult::with_cb(move |s| {
                            if let Some(ref action_cb) = action_cb {
                                action_cb.clone()(s);
                            }
                            s.pop_layer();
                            cb.clone()(s);
                        })
                    }
                    MenuItem::Subtree(_, ref tree) => {
                        let tree = tree.clone();
                        let offset = Vec2::new(10, self.focus + 1);
                        EventResult::with_cb(move |s| {
                            s.screen_mut()
                             .add_layer_at(Position::parent(offset),
                                           MenuPopup::new(tree.clone()));
                        })
                    }
                    _ => panic!("No delimiter here"),
                };
            }

            _ => return EventResult::Ignored,
        }

        self.scrollbase.scroll_to(self.focus);

        EventResult::Consumed(None)
    }

    fn layout(&mut self, size: Vec2) {
        self.scrollbase.set_heights(size.y, self.menu.children.len());
    }
}
