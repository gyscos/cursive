use std::cmp::min;
use std::rc::Rc;
use std::cell::Cell;

use Cursive;
use With;
use menu::MenuTree;
use direction::Direction;
use view::{Position, ScrollBase, View};
use views::{IdView, MenuPopup};
use align::{Align, HAlign, VAlign};
use event::{Callback, Event, EventResult, Key};
use theme::ColorStyle;
use vec::Vec2;
use Printer;

use unicode_width::UnicodeWidthStr;

/// View to select an item among a list.
///
/// It contains a list of values of type T, with associated labels.
///
/// # Examples
///
/// ```
/// # extern crate cursive;
/// # use cursive::prelude::*;
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
///     s.add_layer(Dialog::new(TextView::new(&text))
///                     .button("Quit", |s| s.quit()));
/// });
///
/// let mut siv = Cursive::new();
/// siv.add_layer(Dialog::new(time_select)
///                 .title("How long is your wait?"));
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
        where F: Fn(&mut Cursive, &T) + 'static
    {
        self.on_select = Some(Rc::new(cb));
    }

    /// Sets a callback to be used when an item is selected.
    ///
    ///
    /// Chainable variant.
    pub fn on_select<F>(self, cb: F) -> Self
        where F: Fn(&mut Cursive, &T) + 'static
    {
        self.with(|s| s.set_on_select(cb))
    }

    /// Sets a callback to be used when `<Enter>` is pressed.
    ///
    /// The item currently selected will be given to the callback.
    pub fn set_on_submit<F>(&mut self, cb: F)
        where F: Fn(&mut Cursive, &T) + 'static
    {
        self.on_submit = Some(Rc::new(cb));
    }

    /// Sets a callback to be used when `<Enter>` is pressed.
    ///
    /// The item currently selected will be given to the callback.
    ///
    /// Chainable variant.
    pub fn on_submit<F>(self, cb: F) -> Self
        where F: Fn(&mut Cursive, &T) + 'static
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
        self.items[self.focus()].value.clone()
    }

    /// Adds a item to the list, with given label and value.
    pub fn add_item(&mut self, label: &str, value: T) {
        self.items.push(Item::new(label, value));
    }

    /// Chainable variant of add_item
    pub fn item(mut self, label: &str, value: T) -> Self {
        self.add_item(label, value);

        self
    }

    /// Wraps this view into an IdView with the given id.
    pub fn with_id(self, label: &str) -> IdView<Self> {
        IdView::new(label, self)
    }

    fn draw_item(&self, printer: &Printer, i: usize) {
        let l = self.items[i].label.width();
        let x = self.align.h.get_offset(l, printer.size.x);
        printer.print_hline((0, 0), x, " ");
        printer.print((x, 0), &self.items[i].label);
        if l < printer.size.x {
            printer.print_hline((x + l, 0), printer.size.x - l - x, " ");
        }
    }

    fn focus(&self) -> usize {
        self.focus.get()
    }

    fn focus_up(&mut self, n: usize) {
        let focus = self.focus();
        let n = min(focus, n);
        self.focus.set(focus - n);
    }

    fn focus_down(&mut self, n: usize) {
        let focus = min(self.focus() + n, self.items.len());
        self.focus.set(focus);
    }
}

impl SelectView<String> {
    /// Convenient method to use the label as value.
    pub fn add_item_str(&mut self, label: &str) {
        self.add_item(label, label.to_string());
    }

    /// Chainable variant of add_item_str
    pub fn item_str(self, label: &str) -> Self {
        self.item(label, label.to_string())
    }
}

impl<T: 'static> View for SelectView<T> {
    fn draw(&self, printer: &Printer) {
        self.last_offset.set(printer.offset);

        if self.popup {
            let style = if !self.enabled {
                ColorStyle::Secondary
            } else if !printer.focused {
                ColorStyle::Primary
            } else {
                ColorStyle::Highlight
            };
            let x = printer.size.x;


            printer.with_color(style, |printer| {
                // Prepare the entire background
                printer.print_hline((1, 0), x - 1, " ");
                // Draw the borders
                printer.print((0, 0), "<");
                printer.print((x - 1, 0), ">");

                let label = &self.items[self.focus()].label;

                // And center the text?
                let offset = HAlign::Center.get_offset(label.len(), x);

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
                        printer.with_color(ColorStyle::Secondary, |printer| {
                            self.draw_item(printer, i)
                        });
                    } else {
                        self.draw_item(printer, i);
                    }
                });
            });
        }
    }

    fn get_min_size(&mut self, req: Vec2) -> Vec2 {
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
            let w = if scrolling {
                w + 2
            } else {
                w
            };

            Vec2::new(w, h)
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.popup {
            match event {
                Event::Key(Key::Enter) => {
                    // Build a shallow menu tree to mimick the items array.
                    // TODO: cache it?
                    let mut tree = MenuTree::new();
                    for (i, item) in self.items.iter().enumerate() {
                        let focus = self.focus.clone();
                        let on_submit = self.on_submit.as_ref().cloned();
                        let value = item.value.clone();
                        tree.add_leaf(&item.label, move |s| {
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
                    let text_offset =
                        (self.last_size.x - self.items[focus].label.len()) / 2;
                    // The total offset for the window is:
                    // * the last absolute offset at which we drew this view
                    // * shifted to the top of the focus (so the line matches)
                    // * shifted to the right of the text offset
                    // * shifted top-left of the border+padding of the popup
                    let offset = self.last_offset.get() - (0, focus) +
                                 (text_offset, 0) -
                                 (2, 1);
                    // And now, we can return the callback.
                    EventResult::with_cb(move |s| {
                        // The callback will want to work with a fresh Rc
                        let tree = tree.clone();
                        // We'll relativise the absolute position,
                        // So that we are locked to the parent view.
                        // A nice effect is that window resizes will keep both
                        // layers together.
                        let current_offset = s.screen().offset();
                        let offset = offset - current_offset;
                        // And finally, put the view in view!
                        s.screen_mut()
                            .add_layer_at(Position::parent(offset),
                                          MenuPopup::new(tree).focus(focus));
                    })
                }
                _ => EventResult::Ignored,
            }
        } else {
            match event {
                Event::Key(Key::Up) if self.focus() > 0 => self.focus_up(1),
                Event::Key(Key::Down) if self.focus() + 1 <
                                         self.items.len() => {
                    self.focus_down(1)
                }
                Event::Key(Key::PageUp) => self.focus_up(10),
                Event::Key(Key::PageDown) => self.focus_down(10),
                Event::Key(Key::Home) => self.focus.set(0),
                Event::Key(Key::End) => self.focus.set(self.items.len() - 1),
                Event::Key(Key::Enter) if self.on_submit.is_some() => {
                    let cb = self.on_submit.clone().unwrap();
                    let v = self.selection();
                    // We return a Callback Rc<|s| cb(s, &*v)>
                    return EventResult::Consumed(Some(Callback::from_fn(move |s| {
                        cb(s, &v)
                    })));
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
                        .find(|&(_, item)| item.label.starts_with(c)) {
                        // Apply modulo in case we have a hit
                        // from the chained iterator
                        self.focus.set(i % self.items.len());
                    }
                }
                _ => return EventResult::Ignored,
            }
            let focus = self.focus();
            self.scrollbase.scroll_to(focus);

            EventResult::Consumed(self.on_select.clone().map(|cb| {
                let v = self.selection();
                Callback::from_fn(move |s| cb(s, &v))
            }))
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
    fn new(label: &str, value: T) -> Self {
        Item {
            label: label.to_string(),
            value: Rc::new(value),
        }
    }
}
