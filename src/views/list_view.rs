use Cursive;
use Printer;
use With;
use direction;
use event::{Callback, Event, EventResult, Key};

use std::any::Any;
use std::rc::Rc;
use vec::Vec2;
use view::ScrollBase;
use view::Selector;
use view::View;

enum Child {
    Row(String, Box<View>),
    Delimiter,
}

impl Child {
    fn label(&self) -> &str {
        match *self {
            Child::Row(ref label, _) => label,
            _ => "",
        }
    }

    fn view(&mut self) -> Option<&mut Box<View>> {
        match *self {
            Child::Row(_, ref mut view) => Some(view),
            _ => None,
        }
    }
}

/// Displays a scrollable list of elements.
pub struct ListView {
    children: Vec<Child>,
    scrollbase: ScrollBase,
    focus: usize,
    // This callback is called when the selection is changed.
    on_select: Option<Rc<Fn(&mut Cursive, &String)>>,
}

new_default!(ListView);

impl ListView {
    /// Creates a new, empty `ListView`.
    pub fn new() -> Self {
        ListView {
            children: Vec::new(),
            scrollbase: ScrollBase::new(),
            focus: 0,
            on_select: None,
        }
    }

    /// Adds a view to the end of the list.
    pub fn add_child<V: View + 'static>(&mut self, label: &str, mut view: V) {
        view.take_focus(direction::Direction::none());
        self.children.push(Child::Row(label.to_string(), Box::new(view)));
    }

    /// Removes all children from this view.
    pub fn clear(&mut self) {
        self.children.clear();
        self.focus = 0;
    }

    /// Adds a view to the end of the list.
    ///
    /// Chainable variant.
    pub fn child<V: View + 'static>(self, label: &str, view: V) -> Self {
        self.with(|s| s.add_child(label, view))
    }

    /// Adds a delimiter to the end of the list.
    pub fn add_delimiter(&mut self) {
        self.children.push(Child::Delimiter);
    }

    /// Adds a delimiter to the end of the list.
    ///
    /// Chainable variant.
    pub fn delimiter(self) -> Self {
        self.with(Self::add_delimiter)
    }

    /// Sets a callback to be used when an item is selected.
    pub fn set_on_select<F>(&mut self, cb: F)
        where F: Fn(&mut Cursive, &String) + 'static
    {
        self.on_select = Some(Rc::new(cb));
    }

    /// Sets a callback to be used when an item is selected.
    ///
    /// Chainable variant.
    pub fn on_select<F>(self, cb: F) -> Self
        where F: Fn(&mut Cursive, &String) + 'static
    {
        self.with(|s| s.set_on_select(cb))
    }

    /// Returns the index of the currently focused item.
    ///
    /// Panics if the list is empty.
    pub fn focus(&self) -> usize {
        self.focus
    }

    fn iter_mut<'a>(&'a mut self, from_focus: bool,
                    source: direction::Relative)
                    -> Box<Iterator<Item = (usize, &mut Child)> + 'a> {

        match source {
            direction::Relative::Front => {
                let start = if from_focus { self.focus } else { 0 };

                Box::new(self.children.iter_mut().enumerate().skip(start))
            }
            direction::Relative::Back => {
                let end = if from_focus {
                    self.focus + 1
                } else {
                    self.children.len()
                };
                Box::new(self.children[..end].iter_mut().enumerate().rev())
            }
        }
    }

    fn move_focus(&mut self, n: usize, source: direction::Direction)
                  -> EventResult {
        let i = if let Some(i) =
                       source.relative(direction::Orientation::Vertical)
            .and_then(|rel| {
                // The iterator starts at the focused element.
                // We don't want that one.
                self.iter_mut(true, rel)
                    .skip(1)
                    .filter_map(|p| try_focus(p, source))
                    .take(n)
                    .last()
            }) {
            i
        } else {
            return EventResult::Ignored;
        };
        self.focus = i;
        self.scrollbase.scroll_to(self.focus);

        EventResult::Consumed(self.on_select.clone().map(|cb| {
            let i = self.focus();
            let focused_string = String::from(self.children[i].label());
            Callback::from_fn(move |s| cb(s, &focused_string))
        }))
    }
}

fn try_focus((i, child): (usize, &mut Child), source: direction::Direction)
             -> Option<usize> {
    match *child {
        Child::Delimiter => None,
        Child::Row(_, ref mut view) => {
            if view.take_focus(source) {
                Some(i)
            } else {
                None
            }
        }

    }
}

impl View for ListView {
    fn draw(&self, printer: &Printer) {
        if self.children.is_empty() {
            return;
        }

        let offset = self.children
            .iter()
            .map(Child::label)
            .map(str::len)
            .max()
            .unwrap_or(0) + 1;

        self.scrollbase.draw(printer, |printer, i| {
            match self.children[i] {
                Child::Row(ref label, ref view) => {
                    printer.print((0, 0), label);
                    view.draw(&printer.offset((offset, 0), i == self.focus));
                }
                Child::Delimiter => (),
            }
        });
    }

    fn get_min_size(&mut self, req: Vec2) -> Vec2 {
        let label_size = self.children
            .iter()
            .map(Child::label)
            .map(str::len)
            .max()
            .unwrap_or(0);
        let view_size = self.children
            .iter_mut()
            .filter_map(Child::view)
            .map(|v| v.get_min_size(req).x)
            .max()
            .unwrap_or(0);

        if self.children.len() > req.y {
            Vec2::new(label_size + 1 + view_size + 2, req.y)
        } else {
            Vec2::new(label_size + 1 + view_size, self.children.len())
        }
    }

    fn layout(&mut self, size: Vec2) {
        self.scrollbase.set_heights(size.y, self.children.len());

        let label_size = self.children
            .iter()
            .map(Child::label)
            .map(str::len)
            .max()
            .unwrap_or(0);
        let mut available = size.x - label_size - 1;

        if self.children.len() > size.y {
            available -= 2;
        }

        for child in self.children.iter_mut().filter_map(Child::view) {
            child.layout(Vec2::new(available, 1));
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.children.is_empty() {
            return EventResult::Ignored;
        }

        if let Child::Row(_, ref mut view) = self.children[self.focus] {
            let result = view.on_event(event);
            if result.is_consumed() {
                return result;
            }
        }

        match event {
            Event::Key(Key::Up) if self.focus > 0 => {
                self.move_focus(1, direction::Direction::down())
            }
            Event::Key(Key::Down) if self.focus + 1 < self.children.len() => {
                self.move_focus(1, direction::Direction::up())
            }
            Event::Key(Key::PageUp) => {
                self.move_focus(10, direction::Direction::down())
            }
            Event::Key(Key::PageDown) => {
                self.move_focus(10, direction::Direction::up())
            }
            Event::Key(Key::Home) |
            Event::Ctrl(Key::Home) => {
                self.move_focus(usize::max_value(),
                                direction::Direction::back())
            }
            Event::Key(Key::End) |
            Event::Ctrl(Key::End) => {
                self.move_focus(usize::max_value(),
                                direction::Direction::front())
            }
            Event::Key(Key::Tab) => {
                self.move_focus(1, direction::Direction::front())
            }
            Event::Shift(Key::Tab) => {
                self.move_focus(1, direction::Direction::back())
            }
            _ => EventResult::Ignored,
        }
    }

    fn take_focus(&mut self, source: direction::Direction) -> bool {
        let rel = source.relative(direction::Orientation::Vertical);
        let i = if let Some(i) = self.iter_mut(rel.is_none(),
                      rel.unwrap_or(direction::Relative::Front))
            .filter_map(|p| try_focus(p, source))
            .next() {
            i
        } else {
            // No one wants to be in focus
            return false;
        };
        self.focus = i;
        self.scrollbase.scroll_to(self.focus);
        true
    }

    fn find_any(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.children
            .iter_mut()
            .filter_map(Child::view)
            .filter_map(|v| v.find_any(selector))
            .next()
    }
}
