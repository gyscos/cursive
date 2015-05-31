use color;
use view::{View,IdView,SizeRequest};
use event::{Event,EventResult,Key};
use vec::Vec2;
use printer::Printer;

struct Item<T> {
    label: String,
    value: T,
}

impl <T> Item<T> {
    fn new(label: &str, value: T) -> Self {
        Item {
            label: label.to_string(),
            value: value,
        }
    }
}

/// View to select an item among a list.
///
/// It contains a list of values of type T, with associated labels.
pub struct SelectView<T=String> {
    items: Vec<Item<T>>,
    focus: usize,
}

impl <T> SelectView<T> {
    /// Creates a new empty SelectView.
    pub fn new() -> Self {
        SelectView {
            items: Vec::new(),
            focus: 0,
        }
    }

    /// Returns the value of the currently selected item. Panics if the list is empty.
    pub fn selection(&self) -> &T {
        &self.items[self.focus].value
    }

    /// Adds a item to the list, with given label and value.
    pub fn item(mut self, label: &str, value: T) -> Self {
        self.items.push(Item::new(label,value));

        self
    }

    /// Wraps this view into an IdView with the given id.
    pub fn with_id(self, label: &str) -> IdView<Self> {
        IdView::new(label, self)
    }
}

impl SelectView<String> {
    /// For String-based SelectView, this is a convenient method to use the label as value.
    pub fn item_str(self, label: &str) -> Self {
        self.item(label, label.to_string())
    }
}

impl <T> View for SelectView<T> {
    fn draw(&mut self, printer: &Printer) {
        for (i,item) in self.items.iter().enumerate() {
            let style = if i == self.focus { if printer.focused { color::HIGHLIGHT } else { color::HIGHLIGHT_INACTIVE } } else { color::PRIMARY };
            printer.with_color(style, |printer| printer.print((0,i), &item.label));
        }
    }

    fn get_min_size(&self, _: SizeRequest) -> Vec2 {
        let w = self.items.iter().map(|item| item.label.len()).max().unwrap_or(1);
        let h = self.items.len();
        Vec2::new(w,h)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::KeyEvent(key) => match key {
                Key::Up if self.focus > 0 => self.focus -= 1,
                Key::Down if self.focus + 1 < self.items.len() => self.focus += 1,
                _ => return EventResult::Ignored,
            },
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }

    fn take_focus(&mut self) -> bool {
        true
    }
}
