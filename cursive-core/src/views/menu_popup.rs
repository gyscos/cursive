use crate::{
    align::Align,
    event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent},
    menu,
    rect::Rect,
    style::PaletteStyle,
    view::scroll,
    view::{Position, View},
    views::OnEventView,
    Cursive, Printer, Vec2, With,
};
use std::cmp::min;
use std::sync::Arc;

/// Popup that shows a list of items.
///
/// This is mostly used indirectly when creating a [popup `SelectView`][1] or
/// a [menubar][2].
///
/// [1]: crate::views::SelectView::popup()
/// [2]: crate::Cursive::menubar()
pub struct MenuPopup {
    menu: Arc<menu::Tree>,
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
    ///
    /// The menu tree cannot be modified after this view has been created.
    pub fn new(menu: Arc<menu::Tree>) -> Self {
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
    #[must_use]
    pub fn focus(self, focus: usize) -> Self {
        self.with(|s| s.set_focus(focus))
    }

    /// Returns the position of the currently focused child.
    pub fn get_focus(&self) -> usize {
        self.focus
    }

    fn item_width(item: &menu::Item) -> usize {
        match *item {
            menu::Item::Delimiter => 1,
            menu::Item::Leaf { ref label, .. } => label.width(),
            menu::Item::Subtree { ref label, .. } => label.width() + 3,
        }
    }

    /// Sets the alignment for this view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn align(self, align: Align) -> Self {
        self.with(|s| s.set_align(align))
    }

    /// Sets the alignment for this view.
    pub fn set_align(&mut self, align: Align) {
        self.align = align;
    }

    /// Sets a callback to be used when this view is actively dismissed.
    ///
    /// (When the user hits `<ESC>`)
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_dismiss<F: 'static + Fn(&mut Cursive) + Send + Sync>(self, f: F) -> Self {
        self.with(|s| s.set_on_dismiss(f))
    }

    /// Sets a callback to be used when this view is actively dismissed.
    ///
    /// (When the user hits `<ESC>`)
    pub fn set_on_dismiss<F: 'static + Fn(&mut Cursive) + Send + Sync>(&mut self, f: F) {
        self.on_dismiss = Some(Callback::from_fn(f));
    }

    /// Sets a callback to be used when a leaf is activated.
    ///
    /// Will also be called if a leaf from a subtree is activated.
    ///
    /// Usually used to hide the parent view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_action<F: 'static + Fn(&mut Cursive) + Send + Sync>(self, f: F) -> Self {
        self.with(|s| s.set_on_action(f))
    }

    /// Sets a callback to be used when a leaf is activated.
    ///
    /// Will also be called if a leaf from a subtree is activated.
    ///
    /// Usually used to hide the parent view.
    pub fn set_on_action<F: 'static + Fn(&mut Cursive) + Send + Sync>(&mut self, f: F) {
        self.on_action = Some(Callback::from_fn(f));
    }

    // Scroll up by `n` rows.
    //
    // # Panics
    //
    // If `self.menu.children.is_empty()`.
    fn scroll_up(&mut self, mut n: usize, mut cycle: bool) {
        if self.menu.is_empty() {
            return;
        }

        while n > 0 {
            if self.focus > 0 {
                self.focus -= 1;
            } else if cycle {
                // Only cycle once to prevent endless loop
                cycle = false;
                self.focus = self.menu.children.len() - 1;
            } else {
                break;
            }

            if self.menu.children[self.focus].is_enabled() {
                n -= 1;
            }
        }
    }

    // Scroll down by `n` rows.
    //
    // # Panics
    //
    // If `self.menu.children.is_empty()`.
    fn scroll_down(&mut self, mut n: usize, mut cycle: bool) {
        if self.menu.is_empty() {
            return;
        }

        while n > 0 {
            if self.focus + 1 < self.menu.children.len() {
                self.focus += 1;
            } else if cycle {
                // Only cycle once to prevent endless loop
                cycle = false;
                self.focus = 0;
            } else {
                // Stop if we're at the bottom.
                break;
            }

            if self.menu.children[self.focus].is_enabled() {
                n -= 1;
            }
        }
    }

    // Prepare the callback for when an item has been picked.
    //
    // # Panics
    //
    // If `self.menu.children.is_empty()`.
    fn submit(&mut self) -> EventResult {
        match self.menu.children[self.focus] {
            menu::Item::Leaf { ref cb, .. } => {
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
            menu::Item::Subtree { ref tree, .. } => self.make_subtree_cb(tree),
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

    fn make_subtree_cb(&self, tree: &Arc<menu::Tree>) -> EventResult {
        let tree = Arc::clone(tree);
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
                OnEventView::new(MenuPopup::new(Arc::clone(&tree)).on_action(move |s| {
                    // This will happen when the subtree popup
                    // activates something;
                    // First, remove ourself.
                    s.pop_layer();
                    if let Some(ref action_cb) = action_cb {
                        action_cb.clone()(s);
                    }
                }))
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
        // If there is no item, nothing can be done.
        if self.menu.children.is_empty() {
            return EventResult::Ignored;
        }

        match event {
            Event::Key(Key::Up) => self.scroll_up(1, true),
            Event::Key(Key::PageUp) => self.scroll_up(5, false),
            Event::Key(Key::Down) => self.scroll_down(1, true),
            Event::Key(Key::PageDown) => self.scroll_down(5, false),

            Event::Key(Key::Home) => self.focus = 0,
            Event::Key(Key::End) => self.focus = self.menu.children.len().saturating_sub(1),

            Event::Key(Key::Right) if self.menu.children[self.focus].is_subtree() => {
                return match self.menu.children[self.focus] {
                    menu::Item::Subtree { ref tree, .. } => self.make_subtree_cb(tree),
                    _ => unreachable!("Child is a subtree"),
                };
            }
            Event::Key(Key::Enter) if self.menu.children[self.focus].is_enabled() => {
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
                    if focus < self.menu.len() && self.menu.children[focus].is_enabled() {
                        self.focus = focus;
                    }
                }
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if self.menu.children[self.focus].is_enabled()
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
            return Rect::from_size(Vec2::zero(), size);
        }

        Rect::from_size((0, self.focus), (size.x, 1))
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
        scroll::draw_box_frame(
            self,
            printer,
            |s, y| s.menu.children.get(y).map_or(false, |c| c.is_delimiter()),
            |_s, _x| false,
        );

        // We're giving it a reduced size because of borders.
        let printer = printer.shrinked_centered((2, 2));

        scroll::draw_lines(self, &printer, |s, printer, i| {
            let item = &s.menu.children[i];
            let enabled = printer.enabled && (item.is_enabled() || item.is_delimiter());
            let style = if !enabled {
                PaletteStyle::Secondary
            } else if i == s.focus {
                PaletteStyle::Highlight
            } else {
                PaletteStyle::Primary
            };
            printer.with_style(style, |printer| {
                match *item {
                    menu::Item::Delimiter => {
                        // printer.print_hdelim((0, 0), printer.size.x)
                        printer.print_hline((0, 0), printer.size.x, "─");
                    }
                    menu::Item::Subtree { ref label, .. } => {
                        if printer.size.x < 4 {
                            return;
                        }
                        printer.print_hline((0, 0), printer.size.x, " ");
                        printer.print_styled((1, 0), label);
                        let x = printer.size.x.saturating_sub(3);
                        printer.print((x, 0), ">>");
                    }
                    menu::Item::Leaf { ref label, .. } => {
                        if printer.size.x < 2 {
                            return;
                        }
                        printer.print_hline((0, 0), printer.size.x, " ");
                        printer.print_styled((1, 0), label);
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
                    if !position.fits_in_rect(offset, self.scroll_core.last_outer_size() + (2, 2)) {
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
