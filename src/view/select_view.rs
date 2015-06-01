use std::cmp::min;
use std::rc::Rc;

use color;
use ::Cursive;
use view::{View,IdView,SizeRequest,DimensionRequest};
use event::{Event,EventResult,Key};
use vec::Vec2;
use printer::Printer;
use super::scroll::ScrollBase;

struct Item<T> {
    label: String,
    value: Rc<T>,
}

impl <T> Item<T> {
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
pub struct SelectView<T=String> {
    items: Vec<Item<T>>,
    focus: usize,
    scrollbase: ScrollBase,
    select_cb: Option<Rc<Box<Fn(&mut Cursive,&T)>>>,
}

impl <T: 'static> SelectView<T> {
    /// Creates a new empty SelectView.
    pub fn new() -> Self {
        SelectView {
            items: Vec::new(),
            focus: 0,
            scrollbase: ScrollBase::new(),
            select_cb: None,
        }
    }

    /// Sets a function to be called when an item is selected (when ENTER is pressed).
    pub fn on_select<F>(mut self, cb: F) -> Self
        where F: Fn(&mut Cursive,&T) + 'static
    {
        self.select_cb = Some(Rc::new(Box::new(cb)));

        self
    }

    /// Returns the value of the currently selected item. Panics if the list is empty.
    pub fn selection(&self) -> Rc<T> {
        self.items[self.focus].value.clone()
    }

    /// Adds a item to the list, with given label and value.
    pub fn add_item(&mut self, label: &str, value: T) {
        self.items.push(Item::new(label,value));
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
}

impl SelectView<String> {
    /// For String-based SelectView, this is a convenient method to use the label as value.
    pub fn add_item_str(&mut self, label: &str) {
        self.add_item(label, label.to_string());
    }

    /// Chainable variant of add_item_str
    pub fn item_str(self, label: &str) -> Self {
        self.item(label, label.to_string())
    }

}

impl <T: 'static> View for SelectView<T> {
    fn draw(&mut self, printer: &Printer) {

        self.scrollbase.draw(printer, |printer,i| {
            let style = if i == self.focus {
                if printer.focused { color::HIGHLIGHT }
                else { color::HIGHLIGHT_INACTIVE }
            } else {
                color::PRIMARY
            };
            printer.with_color(style, |printer| printer.print((0,0), &self.items[i].label));
        });
    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        let w = self.items.iter().map(|item| item.label.len()).max().unwrap_or(1);
        let h = self.items.len();

        let scrolling = if let DimensionRequest::Fixed(r_h) = req.h {
            r_h < h
        } else if let DimensionRequest::AtMost(r_h) = req.h {
            r_h < h
        } else {
            false
        };

        let w = if scrolling { w + 1 } else { w };

        Vec2::new(w,h)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::KeyEvent(Key::Up) if self.focus > 0 => self.focus -= 1,
            Event::KeyEvent(Key::Down) if self.focus + 1 < self.items.len() => self.focus += 1,
            Event::KeyEvent(Key::PageUp) => self.focus -= min(self.focus,10),
            Event::KeyEvent(Key::PageDown) => self.focus = min(self.focus+10,self.items.len()-1),
            Event::KeyEvent(Key::Home) => self.focus = 0,
            Event::KeyEvent(Key::End) => self.focus = self.items.len()-1,
            Event::KeyEvent(Key::Enter) if self.select_cb.is_some() => {
                if let Some(ref cb) = self.select_cb {
                    let cb = cb.clone();
                    let v = self.selection();
                    return EventResult::Consumed(Some(Rc::new(Box::new(move |s| cb(s, &*v)))));
                }
            },
            Event::CharEvent(c) => {
                // Starting from the current focus, find the first item that match the char.
                // Cycle back to the beginning of the list when we reach the end.
                // This is achieved by chaining twice the iterator
                let iter = self.items.iter().chain(self.items.iter());
                if let Some((i,_)) = iter.enumerate()
                    .skip(self.focus+1)
                    .find(|&(_,item)| item.label.starts_with(c))
                {
                    // Apply modulo in case we have a hit from the chained iterator
                    self.focus = i % self.items.len();
                }
            },
            _ => return EventResult::Ignored,
        }

        self.scrollbase.scroll_to(self.focus);

        EventResult::Consumed(None)
    }

    fn take_focus(&mut self) -> bool {
        true
    }

    fn layout(&mut self, size: Vec2) {
        self.scrollbase.set_heights(size.y, self.items.len());
    }
}
