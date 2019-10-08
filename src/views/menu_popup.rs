use crate::align::Align;
use crate::event::{
    Callback, Event, EventResult, Key, MouseButton, MouseEvent,
};
use crate::menu::{MenuItem, MenuTree};
use crate::rect::Rect;
use crate::vec::Vec2;
use crate::view::scroll;
use crate::view::{Position, View};
use crate::views::OnEventView;
use crate::Cursive;
use crate::Printer;
use crate::With;
use std::cmp::min;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;

/// Popup that shows a list of items.
///
/// This is mostly used indirectly when creating a [popup `SelectView`][1] or
/// a [menubar][2].
///
/// [1]: crate::views::SelectView::popup()
/// [2]: crate::Cursive::menubar()
pub struct MenuPopup {
    menu: Rc<MenuTree>,
    focus: usize,
    scroll_core: scroll::Core,
    align: Align,
    on_dismiss: Option<Callback>,
    on_action: Option<Callback>,
}

// The `scroll::Scroller` trait is used to weave the borrow phases.
//
// TODO: use some macro to auto-generate this.
impl_scroller!(MenuPopup::scroll_core);

impl MenuPopup {
    /// Creates a new `MenuPopup` using the given menu tree.
    pub fn new(menu: Rc<MenuTree>) -> Self {
        MenuPopup {
            menu,
            focus: 0,
            scroll_core: scroll::Core::new(),
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

    /// Returns the position of the currently focused child.
    pub fn get_focus(&self) -> usize {
        self.focus
    }

    fn item_width(item: &MenuItem) -> usize {
        match *item {
            MenuItem::Delimiter => 1,
            MenuItem::Leaf(ref title, _) => title.width(),
            MenuItem::Subtree(ref title, _) => title.width() + 3,
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
                // Stop if we're at the bottom.
                break;
            }

            if !self.menu.children[self.focus].is_delimiter() {
                n -= 1;
            }
        }
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
            _ => unreachable!("Delimiters cannot be submitted."),
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

    fn make_subtree_cb(&self, tree: &Rc<MenuTree>) -> EventResult {
        let tree = Rc::clone(tree);
        let max_width = 4 + self
            .menu
            .children
            .iter()
            .map(MenuPopup::item_width)
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
                ))
                .on_event(Key::Left, |s| {
                    s.pop_layer();
                }),
            );
        })
    }

    /// Handle an event for the content.
    ///
    /// Here the event has already been relativized. This means `y=0` points to the first item.
    fn inner_on_event(&mut self, event: Event) -> EventResult {
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
                    _ => unreachable!("Child is a subtree"),
                };
            }
            Event::Key(Key::Enter)
                if !self.menu.children[self.focus].is_delimiter() =>
            {
                return self.submit();
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } => {
                // eprintln!("Position: {:?} / {:?}", position, offset);
                if let Some(position) = position.checked_sub(offset) {
                    // Now `position` is relative to the top-left of the content.
                    let focus = position.y;
                    if !self.menu.children[focus].is_delimiter() {
                        self.focus = focus;
                    }
                }
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if !self.menu.children[self.focus].is_delimiter()
                && position
                    .checked_sub(offset)
                    .map(|position| position.y == self.focus)
                    .unwrap_or(false) =>
            {
                return self.submit();
            }
            Event::Key(Key::Esc) => {
                return self.dismiss();
            }

            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }

    /// Compute the required size for the content.
    fn inner_required_size(&mut self, _req: Vec2) -> Vec2 {
        let w = 2 + self
            .menu
            .children
            .iter()
            .map(Self::item_width)
            .max()
            .unwrap_or(1);

        let h = self.menu.children.len();

        Vec2::new(w, h)
    }

    fn inner_important_area(&self, size: Vec2) -> Rect {
        if self.menu.is_empty() {
            return Rect::from((0, 0));
        }

        Rect::from_size((0, self.focus), (size.x, 1))
    }
}

impl View for MenuPopup {
    fn draw(&self, printer: &Printer<'_, '_>) {
        if !printer.size.fits((2, 2)) {
            return;
        }

        let h = self.menu.len();
        // If we're too high, add a vertical offset
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer = &printer.offset((0, offset));

        // Start with a box
        scroll::draw_box_frame(
            self,
            &printer,
            |s, y| s.menu.children[y].is_delimiter(),
            |_s, _x| false,
        );

        // We're giving it a reduced size because of borders.
        let printer = printer.shrinked_centered((2, 2));

        scroll::draw_lines(self, &printer, |s, printer, i| {
            printer.with_selection(i == s.focus, |printer| {
                let item = &s.menu.children[i];
                match *item {
                    MenuItem::Delimiter => {
                        // printer.print_hdelim((0, 0), printer.size.x)
                        printer.print_hline((0, 0), printer.size.x, "─");
                    }
                    MenuItem::Subtree(ref label, _) => {
                        if printer.size.x < 4 {
                            return;
                        }
                        printer.print_hline((0, 0), printer.size.x, " ");
                        printer.print((1, 0), label);
                        let x = printer.size.x.saturating_sub(3);
                        printer.print((x, 0), ">>");
                    }
                    MenuItem::Leaf(ref label, _) => {
                        if printer.size.x < 2 {
                            return;
                        }
                        printer.print_hline((0, 0), printer.size.x, " ");
                        printer.print((1, 0), label);
                    }
                }
            });
        });
    }

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        // We can't really shrink our items here, so it's not flexible.

        // 2 is the padding

        scroll::required_size(
            self,
            req.saturating_sub((2, 2)),
            true,
            Self::inner_required_size,
        ) + (2, 2)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match scroll::on_event(
            self,
            event.relativized((1, 1)),
            Self::inner_on_event,
            Self::inner_important_area,
        ) {
            EventResult::Ignored => {
                // Check back the non-relativized event now
                if let Event::Mouse {
                    event: MouseEvent::Press(_),
                    position,
                    offset,
                } = event
                {
                    // Mouse press will be ignored if they are outside of the content.
                    // They can be on the border, or entirely outside of the popup.

                    // Mouse clicks outside of the popup should dismiss it.
                    if !position.fits_in_rect(
                        offset,
                        self.scroll_core.last_size() + (2, 2),
                    ) {
                        let dismiss_cb = self.on_dismiss.clone();
                        return EventResult::with_cb(move |s| {
                            if let Some(ref cb) = dismiss_cb {
                                cb.clone()(s);
                            }
                            s.pop_layer();
                        });
                    }
                }

                EventResult::Ignored
            }
            other => other,
        }
    }

    fn layout(&mut self, size: Vec2) {
        scroll::layout(
            self,
            size.saturating_sub((2, 2)),
            true,
            |_s, _size| (),
            Self::inner_required_size,
        );
    }

    fn important_area(&self, size: Vec2) -> Rect {
        scroll::important_area(
            self,
            size.saturating_sub((2, 2)),
            Self::inner_important_area,
        )
        .with(|area| area.offset((1, 1)))
    }
}
