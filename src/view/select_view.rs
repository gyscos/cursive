use std::cmp::min;
use std::rc::Rc;

use Cursive;
use With;
use direction::Direction;
use view::{IdView, View};
use align::{Align, HAlign, VAlign};
use view::scroll::ScrollBase;
use event::{Event, EventResult, Key};
use theme::ColorStyle;
use vec::Vec2;
use Printer;

use unicode_width::UnicodeWidthStr;

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

/// View to select an item among a list.
///
/// It contains a list of values of type T, with associated labels.
pub struct SelectView<T = String> {
    items: Vec<Item<T>>,
    enabled: bool,
    focus: usize,
    scrollbase: ScrollBase,
    // This is a custom callback to include a &T
    select_cb: Option<Rc<Fn(&mut Cursive, &T)>>,
    align: Align,
}

impl<T: 'static> SelectView<T> {
    /// Creates a new empty SelectView.
    pub fn new() -> Self {
        SelectView {
            items: Vec::new(),
            enabled: true,
            focus: 0,
            scrollbase: ScrollBase::new(),
            select_cb: None,
            align: Align::top_left(),
        }
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
    ///
    /// (When ENTER is pressed on an item).
    pub fn set_on_select<F>(&mut self, cb: F)
        where F: Fn(&mut Cursive, &T) + 'static
    {
        self.select_cb = Some(Rc::new(cb));
    }

    /// Sets a callback to be used when an item is selected.
    ///
    /// (When ENTER is pressed on an item).
    ///
    /// Chainable variant.
    pub fn on_select<F>(mut self, cb: F) -> Self
        where F: Fn(&mut Cursive, &T) + 'static
    {
        self.set_on_select(cb);

        self
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
        self.items[self.focus].value.clone()
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
        printer.print_hline((x + l, 0), printer.size.x - l - x, " ");
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

        let h = self.items.len();
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer =
            &printer.sub_printer(Vec2::new(0, offset), printer.size, true);

        self.scrollbase.draw(printer, |printer, i| {
            printer.with_selection(i == self.focus, |printer| {
                if i != self.focus && !self.enabled {
                    printer.with_color(ColorStyle::Secondary,
                                       |printer| self.draw_item(printer, i));
                } else {
                    self.draw_item(printer, i);
                }
            });
        });
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

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Up) if self.focus > 0 => self.focus -= 1,
            Event::Key(Key::Down) if self.focus + 1 < self.items.len() => {
                self.focus += 1
            }
            Event::Key(Key::PageUp) => self.focus -= min(self.focus, 10),
            Event::Key(Key::PageDown) => {
                self.focus = min(self.focus + 10, self.items.len() - 1)
            }
            Event::Key(Key::Home) => self.focus = 0,
            Event::Key(Key::End) => self.focus = self.items.len() - 1,
            Event::Key(Key::Enter) if self.select_cb.is_some() => {
                let cb = self.select_cb.as_ref().unwrap().clone();
                let v = self.selection();
                // We return a Callback Rc<|s| cb(s, &*v)>
                return EventResult::Consumed(Some(Rc::new(move |s| {
                    cb(s, &*v)
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
                    .skip(self.focus + 1)
                    .find(|&(_, item)| item.label.starts_with(c)) {
                    // Apply modulo in case we have a hit
                    // from the chained iterator
                    self.focus = i % self.items.len();
                }
            }
            _ => return EventResult::Ignored,
        }

        self.scrollbase.scroll_to(self.focus);

        EventResult::Consumed(None)
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled && !self.items.is_empty()
    }

    fn layout(&mut self, size: Vec2) {
        self.scrollbase.set_heights(size.y, self.items.len());
    }
}
