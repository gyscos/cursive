use Cursive;
use Printer;
use With;
use align::Align;
use event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent};
use menu::{MenuItem, MenuTree};
use rect::Rect;
use std::cmp::min;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;
use vec::Vec2;
use view::{Position, ScrollBase, View};
use views::OnEventView;

/// Popup that shows a list of items.
pub struct MenuPopup {
    menu: Rc<MenuTree>,
    focus: usize,
    scrollbase: ScrollBase,
    align: Align,
    on_dismiss: Option<Callback>,
    on_action: Option<Callback>,
    last_size: Vec2,
}

impl MenuPopup {
    /// Creates a new `MenuPopup` using the given menu tree.
    pub fn new(menu: Rc<MenuTree>) -> Self {
        MenuPopup {
            menu,
            focus: 0,
            scrollbase: ScrollBase::new()
                .scrollbar_offset(1)
                .right_padding(0),
            align: Align::top_left(),
            on_dismiss: None,
            on_action: None,
            last_size: Vec2::zero(),
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
    ///
    /// Chainable variant.
    pub fn align(self, align: Align) -> Self {
        self.with(|s| s.set_align(align))
    }

    /// Sets the alignment for this view.
    pub fn set_align(&mut self, align: Align) {
        self.align = align;
    }

    /// Sets a callback to be used when this view is actively dismissed.
    ///
    /// (When the user hits <ESC>)
    ///
    /// Chainable variant.
    pub fn on_dismiss<F: 'static + Fn(&mut Cursive)>(self, f: F) -> Self {
        self.with(|s| s.set_on_dismiss(f))
    }

    /// Sets a callback to be used when this view is actively dismissed.
    ///
    /// (When the user hits <ESC>)
    pub fn set_on_dismiss<F: 'static + Fn(&mut Cursive)>(&mut self, f: F) {
        self.on_dismiss = Some(Callback::from_fn(f));
    }

    /// Sets a callback to be used when a leaf is activated.
    ///
    /// Will also be called if a leaf from a subtree is activated.
    ///
    /// Usually used to hide the parent view.
    ///
    /// Chainable variant.
    pub fn on_action<F: 'static + Fn(&mut Cursive)>(self, f: F) -> Self {
        self.with(|s| s.set_on_action(f))
    }

    /// Sets a callback to be used when a leaf is activated.
    ///
    /// Will also be called if a leaf from a subtree is activated.
    ///
    /// Usually used to hide the parent view.
    pub fn set_on_action<F: 'static + Fn(&mut Cursive)>(&mut self, f: F) {
        self.on_action = Some(Callback::from_fn(f));
    }

    fn make_subtree_cb(&self, tree: &Rc<MenuTree>) -> EventResult {
        let tree = Rc::clone(tree);
        let max_width = 4 + self.menu
            .children
            .iter()
            .map(Self::item_width)
            .max()
            .unwrap_or(1);
        let offset = Vec2::new(max_width, self.focus);
        let action_cb = self.on_action.clone();

        EventResult::with_cb(move |s| {
            let action_cb = action_cb.clone();
            s.screen_mut().add_layer_at(
                Position::parent(offset),
                OnEventView::new(MenuPopup::new(Rc::clone(&tree)).on_action(
                    move |s| {
                        // This will happen when the subtree popup
                        // activates something;
                        // First, remove ourselve.
                        s.pop_layer();
                        if let Some(ref action_cb) = action_cb {
                            action_cb.clone()(s);
                        }
                    },
                )).on_event(Key::Left, |s| {
                    s.pop_layer();
                }),
            );
        })
    }

    fn submit(&mut self) -> EventResult {
        match self.menu.children[self.focus] {
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
            MenuItem::Subtree(_, ref tree) => self.make_subtree_cb(tree),
            _ => panic!("No delimiter here"),
        }
    }

    fn dismiss(&mut self) -> EventResult {
        let dismiss_cb = self.on_dismiss.clone();
        EventResult::with_cb(move |s| {
            if let Some(ref cb) = dismiss_cb {
                cb.clone()(s);
            }
            s.pop_layer();
        })
    }
}

impl View for MenuPopup {
    fn draw(&self, printer: &Printer) {
        if !printer.size.fits((2, 2)) {
            return;
        }

        let h = self.menu.len();
        // If we're too high, add a vertical offset
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer = &printer.offset((0, offset));

        // Start with a box
        printer.print_box(Vec2::new(0, 0), printer.size, false);

        // We're giving it a reduced size because of borders.
        // But we're keeping the full width,
        // to integrate horizontal delimiters in the frame.
        let printer = printer.offset((0, 1)).shrinked((0, 1));

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
                        let x = printer.size.x.saturating_sub(4);
                        printer.print((x, 0), ">>");
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

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        // We can't really shrink our items here, so it's not flexible.
        let w = 4 + self.menu
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
        let mut fix_scroll = true;
        match event {
            Event::Key(Key::Up) => self.scroll_up(1, true),
            Event::Key(Key::PageUp) => self.scroll_up(5, false),
            Event::Key(Key::Down) => self.scroll_down(1, true),
            Event::Key(Key::PageDown) => self.scroll_down(5, false),

            Event::Key(Key::Home) => self.focus = 0,
            Event::Key(Key::End) => {
                self.focus = self.menu.children.len().saturating_sub(1)
            }

            Event::Key(Key::Right)
                if self.menu.children[self.focus].is_subtree() =>
            {
                return match self.menu.children[self.focus] {
                    MenuItem::Subtree(_, ref tree) => {
                        self.make_subtree_cb(tree)
                    }
                    _ => panic!("Not a subtree???"),
                };
            }
            Event::Key(Key::Enter)
                if !self.menu.children[self.focus].is_delimiter() =>
            {
                return self.submit();
            }
            Event::Mouse {
                event: MouseEvent::WheelUp,
                ..
            } if self.scrollbase.can_scroll_up() =>
            {
                fix_scroll = false;
                self.scrollbase.scroll_up(1);
            }
            Event::Mouse {
                event: MouseEvent::WheelDown,
                ..
            } if self.scrollbase.can_scroll_down() =>
            {
                fix_scroll = false;
                self.scrollbase.scroll_down(1);
            }
            Event::Mouse {
                event: MouseEvent::Press(MouseButton::Left),
                position,
                offset,
            } if self.scrollbase.scrollable()
                && position
                    .checked_sub(offset + (0, 1))
                    .map(|position| {
                        self.scrollbase
                            .start_drag(position, self.last_size.x)
                    })
                    .unwrap_or(false) =>
            {
                fix_scroll = false;
            }
            Event::Mouse {
                event: MouseEvent::Hold(MouseButton::Left),
                position,
                offset,
            } => {
                // If the mouse is dragged, we always consume the event.
                fix_scroll = false;
                let position = position.saturating_sub(offset + (0, 1));
                self.scrollbase.drag(position);
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } if position.fits_in_rect(offset, self.last_size) =>
            {
                // eprintln!("Position: {:?} / {:?}", position, offset);
                // eprintln!("Last size: {:?}", self.last_size);
                let inner_size = self.last_size.saturating_sub((2, 2));
                position.checked_sub(offset + (1, 1)).map(
                    // `position` is not relative to the content
                    // (It's inside the border)
                    |position| {
                        if position < inner_size {
                            let focus =
                                position.y + self.scrollbase.start_line;
                            if !self.menu.children[focus].is_delimiter() {
                                self.focus = focus;
                            }
                        }
                    },
                );
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } => {
                fix_scroll = false;
                self.scrollbase.release_grab();
                if !self.menu.children[self.focus].is_delimiter() {
                    if let Some(position) =
                        position.checked_sub(offset + (1, 1))
                    {
                        if position < self.last_size.saturating_sub((2, 2))
                            && (position.y + self.scrollbase.start_line
                                == self.focus)
                        {
                            return self.submit();
                        }
                    }
                }
            }
            Event::Key(Key::Esc)
            | Event::Mouse {
                event: MouseEvent::Press(_),
                ..
            } => {
                return self.dismiss();
            }

            _ => return EventResult::Ignored,
        }

        if fix_scroll {
            self.scrollbase.scroll_to(self.focus);
        }

        EventResult::Consumed(None)
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;
        self.scrollbase.set_heights(
            size.y.saturating_sub(2),
            self.menu.children.len(),
        );
    }

    fn important_area(&self, size: Vec2) -> Rect {
        if self.menu.is_empty() {
            return Rect::from((0, 0));
        }

        Rect::from_size((0, self.focus), (size.x, 1))
    }
}
