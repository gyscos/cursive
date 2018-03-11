use Cursive;
use Printer;
use With;
use align::{Align, HAlign, VAlign};
use direction::Direction;
use event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent};
use menu::MenuTree;
use std::borrow::Borrow;
use std::cell::Cell;
use std::cmp::min;
use std::rc::Rc;
use theme::ColorStyle;
use unicode_width::UnicodeWidthStr;
use vec::Vec2;
use view::{Position, ScrollBase, View};
use views::MenuPopup;

/// View to select an item among a list.
///
/// It contains a list of values of type T, with associated labels.
///
/// # Examples
///
/// ```no_run
/// # extern crate cursive;
/// # use cursive::Cursive;
/// # use cursive::views::{SelectView, Dialog, TextView};
/// # use cursive::align::HAlign;
/// # fn main() {
/// let mut time_select = SelectView::new().h_align(HAlign::Center);
/// time_select.add_item("Short", 1);
/// time_select.add_item("Medium", 5);
/// time_select.add_item("Long", 10);
///
/// time_select.set_on_submit(|s, time| {
///     s.pop_layer();
///     let text = format!("You will wait for {} minutes...", time);
///     s.add_layer(Dialog::around(TextView::new(text))
///                     .button("Quit", |s| s.quit()));
/// });
///
/// let mut siv = Cursive::new();
/// siv.add_layer(Dialog::around(time_select)
///                      .title("How long is your wait?"));
/// # }
///
/// ```
pub struct SelectView<T = String> {
    items: Vec<Item<T>>,
    enabled: bool,
    // the focus needs to be manipulable from callbacks
    focus: Rc<Cell<usize>>,
    scrollbase: ScrollBase,
    // This is a custom callback to include a &T.
    // It will be called whenever "Enter" is pressed.
    on_submit: Option<Rc<Fn(&mut Cursive, &T)>>,
    // This callback is called when the selection is changed.
    on_select: Option<Rc<Fn(&mut Cursive, &T)>>,
    align: Align,
    // `true` if we show a one-line view, with popup on selection.
    popup: bool,
    // We need the last offset to place the popup window
    // We "cache" it during the draw, so we need interior mutability.
    last_offset: Cell<Vec2>,
    last_size: Vec2,
}

impl<T: 'static> Default for SelectView<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> SelectView<T> {
    /// Creates a new empty SelectView.
    pub fn new() -> Self {
        SelectView {
            items: Vec::new(),
            enabled: true,
            focus: Rc::new(Cell::new(0)),
            scrollbase: ScrollBase::new(),
            on_select: None,
            on_submit: None,
            align: Align::top_left(),
            popup: false,
            last_offset: Cell::new(Vec2::zero()),
            last_size: Vec2::zero(),
        }
    }

    /// Turns `self` into a popup select view.
    ///
    /// Chainable variant.
    pub fn popup(self) -> Self {
        self.with(|s| s.set_popup(true))
    }

    /// Turns `self` into a popup select view.
    pub fn set_popup(&mut self, popup: bool) {
        self.popup = popup;
    }

    /// Disables this view.
    ///
    /// A disabled view cannot be selected.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Disables this view.
    ///
    /// Chainable variant.
    pub fn disabled(self) -> Self {
        self.with(Self::disable)
    }

    /// Re-enables this view.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Enable or disable this view.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns `true` if this view is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets a callback to be used when an item is selected.
    pub fn set_on_select<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive, &T) + 'static,
    {
        self.on_select = Some(Rc::new(cb));
    }

    /// Sets a callback to be used when an item is selected.
    ///
    /// Chainable variant.
    pub fn on_select<F>(self, cb: F) -> Self
    where
        F: Fn(&mut Cursive, &T) + 'static,
    {
        self.with(|s| s.set_on_select(cb))
    }

    /// Sets a callback to be used when `<Enter>` is pressed.
    ///
    /// The item currently selected will be given to the callback.
    ///
    /// Here, `V` can be `T` itself, or a type that can be borrowed from `T`.
    pub fn set_on_submit<F, R, V: ?Sized>(&mut self, cb: F)
    where
        F: 'static + Fn(&mut Cursive, &V) -> R,
        T: Borrow<V>,
    {
        self.on_submit = Some(Rc::new(move |s, t| {
            cb(s, t.borrow());
        }));
    }

    /// Sets a callback to be used when `<Enter>` is pressed.
    ///
    /// The item currently selected will be given to the callback.
    ///
    /// Chainable variant.
    pub fn on_submit<F, V: ?Sized>(self, cb: F) -> Self
    where
        F: Fn(&mut Cursive, &V) + 'static,
        T: Borrow<V>,
    {
        self.with(|s| s.set_on_submit(cb))
    }

    /// Sets the alignment for this view.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;

        self
    }

    /// Sets the vertical alignment for this view.
    /// (If the view is given too much space vertically.)
    pub fn v_align(mut self, v: VAlign) -> Self {
        self.align.v = v;

        self
    }

    /// Sets the horizontal alignment for this view.
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
    }

    /// Returns the value of the currently selected item.
    ///
    /// Panics if the list is empty.
    pub fn selection(&self) -> Rc<T> {
        Rc::clone(&self.items[self.focus()].value)
    }

    /// Removes all items from this view.
    pub fn clear(&mut self) {
        self.items.clear();
        self.focus.set(0);
    }

    /// Adds a item to the list, with given label and value.
    pub fn add_item<S: Into<String>>(&mut self, label: S, value: T) {
        self.items.push(Item::new(label.into(), value));
    }

    /// Gets an item at given idx or None.
    ///
    /// ```
    /// use cursive::Cursive;
    /// use cursive::views::{SelectView, TextView};
    /// let select = SelectView::new()
    ///     .item("Short", 1);
    /// assert_eq!(select.get_item(0), Some(("Short", &1)));
    /// ```
    pub fn get_item(&self, i: usize) -> Option<(&str, &T)> {
        self.items
            .get(i)
            .map(|item| (item.label.as_ref(), &*item.value))
    }

    /// Gets a mut item at given idx or None.
    pub fn get_item_mut(&mut self, i: usize) -> Option<(&mut String, &mut T)> {
        if i >= self.items.len() {
            None
        } else {
            let item = &mut self.items[i];
            if let Some(t) = Rc::get_mut(&mut item.value) {
                let label = &mut item.label;
                Some((label, t))
            } else {
                None
            }
        }
    }

    /// Removes an item from the list.
    pub fn remove_item(&mut self, id: usize) {
        self.items.remove(id);
        let focus = self.focus();
        if focus >= id && focus > 0 {
            self.focus.set(focus - 1);
        }
    }

    /// Chainable variant of add_item
    pub fn item<S: Into<String>>(self, label: S, value: T) -> Self {
        self.with(|s| s.add_item(label, value))
    }

    /// Adds all items from from an iterator.
    pub fn add_all<S, I>(&mut self, iter: I)
    where
        S: Into<String>,
        I: IntoIterator<Item = (S, T)>,
    {
        for (s, t) in iter {
            self.add_item(s, t);
        }
    }

    /// Adds all items from from an iterator.
    ///
    /// Chainable variant.
    pub fn with_all<S, I>(self, iter: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator<Item = (S, T)>,
    {
        self.with(|s| s.add_all(iter))
    }

    fn draw_item(&self, printer: &Printer, i: usize) {
        let l = self.items[i].label.width();
        let x = self.align.h.get_offset(l, printer.size.x);
        printer.print_hline((0, 0), x, " ");
        printer.print((x, 0), &self.items[i].label);
        if l < printer.size.x {
            assert!((l + x) <= printer.size.x);
            printer.print_hline((x + l, 0), printer.size.x - (l + x), " ");
        }
    }

    /// Returns the id of the item currently selected.
    ///
    /// Returns `None` if the list is empty.
    pub fn selected_id(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.focus())
        }
    }

    /// Returns the number of items in this list.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if this list has no item.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn focus(&self) -> usize {
        self.focus.get()
    }

    /// Moves the selection to the given position.
    pub fn set_selection(&mut self, i: usize) {
        // TODO: Check if `i > self.len()` ?
        self.focus.set(i);
        self.scrollbase.scroll_to(i);
    }

    /// Sets the selection to the given position.
    ///
    /// Chainable variant.
    pub fn selected(self, i: usize) -> Self {
        self.with(|s| s.set_selection(i))
    }

    /// Moves the selection up by the given number of rows.
    pub fn select_up(&mut self, n: usize) {
        self.focus_up(n);
        let focus = self.focus();
        self.scrollbase.scroll_to(focus);
    }

    /// Moves the selection down by the given number of rows.
    pub fn select_down(&mut self, n: usize) {
        self.focus_down(n);
        let focus = self.focus();
        self.scrollbase.scroll_to(focus);
    }

    // Low-level focus change. Does not fix scrollbase.
    fn focus_up(&mut self, n: usize) {
        let focus = self.focus().saturating_sub(n);
        self.focus.set(focus);
    }

    // Low-level focus change. Does not fix scrollbase.
    fn focus_down(&mut self, n: usize) {
        let focus = min(self.focus() + n, self.items.len().saturating_sub(1));
        self.focus.set(focus);
    }

    fn submit(&mut self) -> EventResult {
        let cb = self.on_submit.clone().unwrap();
        let v = self.selection();
        // We return a Callback Rc<|s| cb(s, &*v)>
        EventResult::Consumed(Some(Callback::from_fn(move |s| cb(s, &v))))
    }

    fn on_event_regular(&mut self, event: Event) -> EventResult {
        let mut fix_scroll = true;
        match event {
            Event::Key(Key::Up) if self.focus() > 0 => self.focus_up(1),
            Event::Key(Key::Down) if self.focus() + 1 < self.items.len() => {
                self.focus_down(1)
            }
            Event::Key(Key::PageUp) => self.focus_up(10),
            Event::Key(Key::PageDown) => self.focus_down(10),
            Event::Key(Key::Home) => self.focus.set(0),
            Event::Key(Key::End) => {
                self.focus.set(self.items.len().saturating_sub(1))
            }
            Event::Mouse {
                event: MouseEvent::WheelDown,
                ..
            } if self.scrollbase.can_scroll_down() =>
            {
                fix_scroll = false;
                self.scrollbase.scroll_down(5);
            }
            Event::Mouse {
                event: MouseEvent::WheelUp,
                ..
            } if self.scrollbase.can_scroll_up() =>
            {
                fix_scroll = false;
                self.scrollbase.scroll_up(5);
            }
            Event::Mouse {
                event: MouseEvent::Press(MouseButton::Left),
                position,
                offset,
            } if position
                .checked_sub(offset)
                .map(|position| {
                    self.scrollbase.start_drag(position, self.last_size.x)
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
                let position = position.saturating_sub(offset);
                self.scrollbase.drag(position);
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } => if let Some(position) = position.checked_sub(offset) {
                let scrollbar_size = if self.scrollbase.scrollable() {
                    (2, 0)
                } else {
                    (0, 0)
                };
                let clickable_size =
                    self.last_size.saturating_sub(scrollbar_size);
                if position < clickable_size {
                    fix_scroll = false;
                    self.focus.set(position.y + self.scrollbase.start_line);
                }
            },
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } => {
                fix_scroll = false;
                self.scrollbase.release_grab();
                if self.on_submit.is_some() {
                    if let Some(position) = position.checked_sub(offset) {
                        let scrollbar_size = if self.scrollbase.scrollable() {
                            (2, 0)
                        } else {
                            (0, 0)
                        };
                        let clickable_size =
                            self.last_size.saturating_sub(scrollbar_size);
                        if position < clickable_size
                            && (position.y + self.scrollbase.start_line)
                                == self.focus()
                        {
                            return self.submit();
                        }
                    }
                }
            }
            Event::Key(Key::Enter) if self.on_submit.is_some() => {
                return self.submit();
            }
            Event::Char(c) => {
                // Starting from the current focus,
                // find the first item that match the char.
                // Cycle back to the beginning of
                // the list when we reach the end.
                // This is achieved by chaining twice the iterator
                let iter = self.items.iter().chain(self.items.iter());
                if let Some((i, _)) = iter.enumerate()
                    .skip(self.focus() + 1)
                    .find(|&(_, item)| item.label.starts_with(c))
                {
                    // Apply modulo in case we have a hit
                    // from the chained iterator
                    self.focus.set(i % self.items.len());
                } else {
                    return EventResult::Ignored;
                }
            }
            _ => return EventResult::Ignored,
        }
        if fix_scroll {
            let focus = self.focus();
            self.scrollbase.scroll_to(focus);
        }

        EventResult::Consumed(self.on_select.clone().map(|cb| {
            let v = self.selection();
            Callback::from_fn(move |s| cb(s, &v))
        }))
    }

    fn open_popup(&mut self) -> EventResult {
        // Build a shallow menu tree to mimick the items array.
        // TODO: cache it?
        let mut tree = MenuTree::new();
        for (i, item) in self.items.iter().enumerate() {
            let focus = Rc::clone(&self.focus);
            let on_submit = self.on_submit.as_ref().cloned();
            let value = Rc::clone(&item.value);
            tree.add_leaf(item.label.clone(), move |s| {
                focus.set(i);
                if let Some(ref on_submit) = on_submit {
                    on_submit(s, &value);
                }
            });
        }
        // Let's keep the tree around,
        // the callback will want to use it.
        let tree = Rc::new(tree);

        let focus = self.focus();
        // This is the offset for the label text.
        // We'll want to show the popup so that the text matches.
        // It'll be soo cool.
        let item_length = self.items[focus].label.len();
        let text_offset = (self.last_size.x.saturating_sub(item_length)) / 2;
        // The total offset for the window is:
        // * the last absolute offset at which we drew this view
        // * shifted to the right of the text offset
        // * shifted to the top of the focus (so the line matches)
        // * shifted top-left of the border+padding of the popup
        let offset = self.last_offset.get();
        let offset = offset + (text_offset, 0);
        let offset = offset.saturating_sub((0, focus));
        let offset = offset.saturating_sub((2, 1));

        // And now, we can return the callback that will create the popup.
        EventResult::with_cb(move |s| {
            // The callback will want to work with a fresh Rc
            let tree = Rc::clone(&tree);
            // We'll relativise the absolute position,
            // So that we are locked to the parent view.
            // A nice effect is that window resizes will keep both
            // layers together.
            let current_offset = s.screen().offset();
            let offset = offset.signed() - current_offset;
            // And finally, put the view in view!
            s.screen_mut().add_layer_at(
                Position::parent(offset),
                MenuPopup::new(tree).focus(focus),
                None,
            );
        })
    }

    // A popup view only does one thing: open the popup on Enter.
    fn on_event_popup(&mut self, event: Event) -> EventResult {
        match event {
            // TODO: add Left/Right support for quick-switch?
            Event::Key(Key::Enter) => self.open_popup(),
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset, self.last_size) =>
            {
                self.open_popup()
            }
            _ => EventResult::Ignored,
        }
    }
}

impl SelectView<String> {
    /// Convenient method to use the label as value.
    pub fn add_item_str<S: Into<String>>(&mut self, label: S) {
        let label = label.into();
        self.add_item(label.clone(), label);
    }

    /// Chainable variant of add_item_str
    pub fn item_str<S: Into<String>>(self, label: S) -> Self {
        self.with(|s| s.add_item_str(label))
    }

    /// Adds all strings from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cursive::views::SelectView;
    /// let mut select_view = SelectView::new();
    /// select_view.add_all_str(vec!["a", "b", "c"]);
    /// ```
    pub fn add_all_str<S, I>(&mut self, iter: I)
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        for s in iter {
            self.add_item_str(s);
        }
    }

    /// Adds all strings from an iterator.
    ///
    /// Chainable variant.
    pub fn with_all_str<S, I>(self, iter: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        self.with(|s| s.add_all_str(iter))
    }
}

impl<T: 'static> View for SelectView<T> {
    fn draw(&self, printer: &Printer) {
        self.last_offset.set(printer.offset);

        if self.popup {
            let style = if !self.enabled {
                ColorStyle::secondary()
            } else if !printer.focused {
                ColorStyle::primary()
            } else {
                ColorStyle::highlight()
            };
            let x = match printer.size.x.checked_sub(1) {
                Some(x) => x,
                None => return,
            };

            printer.with_color(style, |printer| {
                // Prepare the entire background
                printer.print_hline((1, 0), x, " ");
                // Draw the borders
                printer.print((0, 0), "<");
                printer.print((x, 0), ">");

                let label = &self.items[self.focus()].label;

                // And center the text?
                let offset = HAlign::Center.get_offset(label.len(), x + 1);

                printer.print((offset, 0), label);
            });
        } else {
            let h = self.items.len();
            let offset = self.align.v.get_offset(h, printer.size.y);
            let printer =
                &printer.sub_printer(Vec2::new(0, offset), printer.size, true);

            self.scrollbase.draw(printer, |printer, i| {
                printer.with_selection(i == self.focus(), |printer| {
                    if i != self.focus() && !self.enabled {
                        printer
                            .with_color(ColorStyle::secondary(), |printer| {
                                self.draw_item(printer, i)
                            });
                    } else {
                        self.draw_item(printer, i);
                    }
                });
            });
        }
    }

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        // Items here are not compressible.
        // So no matter what the horizontal requirements are,
        // we'll still return our longest item.
        let w = self.items
            .iter()
            .map(|item| item.label.width())
            .max()
            .unwrap_or(1);
        if self.popup {
            Vec2::new(w + 2, 1)
        } else {
            let h = self.items.len();

            let scrolling = req.y < h;

            // Add 2 spaces for the scrollbar if we need
            let w = if scrolling { w + 2 } else { w };

            // Don't request more than we're offered - we can scroll,
            // after all
            Vec2::new(w, min(h, req.y))
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.popup {
            self.on_event_popup(event)
        } else {
            self.on_event_regular(event)
        }
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled && !self.items.is_empty()
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;

        if !self.popup {
            self.scrollbase.set_heights(size.y, self.items.len());
        }
    }
}

struct Item<T> {
    label: String,
    value: Rc<T>,
}

impl<T> Item<T> {
    fn new(label: String, value: T) -> Self {
        Item {
            label: label,
            value: Rc::new(value),
        }
    }
}
