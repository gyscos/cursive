

use Cursive;
use Printer;
use With;
use align::Align;
use event::{Callback, Event, EventResult, Key};
use menu::{MenuItem, MenuTree};
use std::cmp::min;
use std::rc::Rc;

use unicode_width::UnicodeWidthStr;
use vec::Vec2;
use view::{Position, ScrollBase, View};
use views::KeyEventView;

/// Popup that shows a list of items.
pub struct MenuPopup {
    menu: Rc<MenuTree>,
    focus: usize,
    scrollbase: ScrollBase,
    align: Align,
    on_dismiss: Option<Callback>,
    on_action: Option<Callback>,
}

impl MenuPopup {
    /// Creates a new `MenuPopup` using the given menu tree.
    pub fn new(menu: Rc<MenuTree>) -> Self {
        MenuPopup {
            menu: menu,
            focus: 0,
            scrollbase: ScrollBase::new().scrollbar_offset(1).right_padding(0),
            align: Align::top_left(),
            on_dismiss: None,
            on_action: None,
        }
    }

    /// Sets the currently focused element.
    pub fn set_focus(&mut self, focus: usize) {
        self.focus = min(focus, self.menu.len());
    }

    /// Sets the currently focused element.
    ///
    /// Chainable variant.
    pub fn focus(self, focus: usize) -> Self {
        self.with(|s| s.set_focus(focus))
    }

    fn item_width(item: &MenuItem) -> usize {
        match *item {
            MenuItem::Delimiter => 1,
            MenuItem::Leaf(ref title, _) => title.width(),
            MenuItem::Subtree(ref title, _) => title.width() + 3,
        }
    }

    fn scroll_up(&mut self, mut n: usize, cycle: bool) {
        while n > 0 {
            if self.focus > 0 {
                self.focus -= 1;
            } else if cycle {
                self.focus = self.menu.children.len() - 1;
            } else {
                break;
            }

            if !self.menu.children[self.focus].is_delimiter() {
                n -= 1;
            }
        }
    }

    fn scroll_down(&mut self, mut n: usize, cycle: bool) {
        while n > 0 {
            if self.focus + 1 < self.menu.children.len() {
                self.focus += 1;
            } else if cycle {
                self.focus = 0;
            } else {
                break;
            }
            if !self.menu.children[self.focus].is_delimiter() {
                n -= 1;
            }
        }
    }


    /// Sets the alignment for this view.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;

        self
    }

    /// Sets a callback to be used when this view is actively dismissed.
    ///
    /// (When the user hits <ESC>)
    pub fn on_dismiss<F: 'static + Fn(&mut Cursive)>(mut self, f: F) -> Self {
        self.on_dismiss = Some(Callback::from_fn(f));
        self
    }

    /// Sets a callback to be used when a leaf is activated.
    ///
    /// Will also be called if a leaf from a subtree is activated.
    ///
    /// Usually used to hide the parent view.
    pub fn on_action<F: 'static + Fn(&mut Cursive)>(mut self, f: F) -> Self {
        self.on_action = Some(Callback::from_fn(f));
        self
    }

    fn make_subtree_cb(&self, tree: &Rc<MenuTree>) -> EventResult {
        let tree = tree.clone();
        let max_width = 4 +
                        self.menu
            .children
            .iter()
            .map(Self::item_width)
            .max()
            .unwrap_or(1);
        let offset = Vec2::new(max_width, self.focus);
        let action_cb = self.on_action.clone();

        EventResult::with_cb(move |s| {
            let action_cb = action_cb.clone();
            s.screen_mut()
                .add_layer_at(Position::parent(offset),
                              KeyEventView::new(MenuPopup::new(tree.clone())
                                      .on_action(move |s| {
                            // This will happen when the subtree popup
                            // activates something;
                            // First, remove ourselve.
                            s.pop_layer();
                            if let Some(ref action_cb) = action_cb {
                                action_cb.clone()(s);
                            }
                        }))
                                  .register(Key::Left, |s| s.pop_layer()));
        })
    }
}

impl View for MenuPopup {
    fn draw(&self, printer: &Printer) {
        if printer.size.x < 2 || printer.size.y < 2 {
            return;
        }

        let h = self.menu.len();
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer =
            &printer.sub_printer(Vec2::new(0, offset), printer.size, true);

        // Start with a box
        printer.print_box(Vec2::new(0, 0), printer.size, false);

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
                    MenuItem::Subtree(ref label, _) => {
                        if printer.size.x < 4 {
                            return;
                        }
                        printer.print_hline((1, 0), printer.size.x - 2, " ");
                        printer.print((2, 0), label);
                        printer.print((printer.size.x - 4, 0), ">>");
                    }
                    MenuItem::Leaf(ref label, _) => {
                        if printer.size.x < 2 {
                            return;
                        }
                        printer.print_hline((1, 0), printer.size.x - 2, " ");
                        printer.print((2, 0), label);
                    }
                }

            });
        });
    }

    fn get_min_size(&mut self, req: Vec2) -> Vec2 {
        // We can't really shrink our items here, so it's not flexible.
        let w = 4 +
                self.menu
            .children
            .iter()
            .map(Self::item_width)
            .max()
            .unwrap_or(1);
        let h = 2 + self.menu.children.len();


        let scrolling = req.y < h;

        let w = if scrolling { w + 1 } else { w };

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
            Event::Key(Key::Up) => self.scroll_up(1, true),
            Event::Key(Key::PageUp) => self.scroll_up(5, false),
            Event::Key(Key::Down) => self.scroll_down(1, true),
            Event::Key(Key::PageDown) => self.scroll_down(5, false),

            Event::Key(Key::Home) => self.focus = 0,
            Event::Key(Key::End) => {
                self.focus = self.menu.children.len() - 1
            }

            Event::Key(Key::Right) if self.menu.children
                                          [self.focus]
                .is_subtree() => {
                return match self.menu.children[self.focus] {
                    MenuItem::Subtree(_, ref tree) => {
                        self.make_subtree_cb(tree)
                    }
                    _ => panic!("Not a subtree???"),

                };
            }
            Event::Key(Key::Enter) if !self.menu.children
                                           [self.focus]
                .is_delimiter() => {
                return match self.menu.children[self.focus] {
                    MenuItem::Leaf(_, ref cb) => {

                        let cb = cb.clone();
                        let action_cb = self.on_action.clone();
                        EventResult::with_cb(move |s| {
                            // Remove ourselves from the face of the earth
                            s.pop_layer();
                            // If we had prior orders, do it now.
                            if let Some(ref action_cb) = action_cb {
                                action_cb.clone()(s);
                            }
                            // And transmit his last words.
                            cb.clone()(s);
                        })
                    }
                    MenuItem::Subtree(_, ref tree) => {
                        self.make_subtree_cb(tree)
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
        self.scrollbase
            .set_heights(size.y - 2, self.menu.children.len());
    }
}
