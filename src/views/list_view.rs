use Cursive;
use Printer;
use With;
use direction;
use event::{Callback, Event, EventResult, Key};

use std::any::Any;
use std::rc::Rc;

use unicode_width::UnicodeWidthStr;
use vec::Vec2;
use view::ScrollBase;
use view::Selector;
use view::View;

/// Represents a child from a [`ListView`].
///
/// [`ListView`]: struct.ListView.html
pub enum ListChild {
    /// A single row, with a label and a view.
    Row(String, Box<View>),
    /// A delimiter between groups.
    Delimiter,
}

impl ListChild {
    fn label(&self) -> &str {
        match *self {
            ListChild::Row(ref label, _) => label,
            _ => "",
        }
    }

    fn view(&mut self) -> Option<&mut View> {
        match *self {
            ListChild::Row(_, ref mut view) => Some(view.as_mut()),
            _ => None,
        }
    }
}

/// Displays a scrollable list of elements.
pub struct ListView {
    children: Vec<ListChild>,
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

    /// Returns the number of children, including delimiters.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if this view contains no children.
    ///
    /// Returns `false` if at least a delimiter or a view is present.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }


    /// Returns a reference to the children
    pub fn children(&self) -> &[ListChild] {
        &self.children[..]
    }

    /// Returns a reference to the child at the given position.
    pub fn get_row(&self, id: usize) -> &ListChild {
        &self.children[id]
    }

    /// Gives mutable access to the child at the given position.
    ///
    /// # Panics
    ///
    /// Panics if `id >= self.len()`.
    pub fn row_mut(&mut self, id: usize) -> &mut ListChild {
        &mut self.children[id]
    }

    /// Adds a view to the end of the list.
    pub fn add_child<V: View + 'static>(&mut self, label: &str, mut view: V) {
        view.take_focus(direction::Direction::none());
        self.children.push(ListChild::Row(label.to_string(), Box::new(view)));
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
        self.children.push(ListChild::Delimiter);
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
                    -> Box<Iterator<Item = (usize, &mut ListChild)> + 'a> {

        match source {
            direction::Relative::Front => {
                let start = if from_focus { self.focus } else { 0 };

                Box::new(self.children
                             .iter_mut()
                             .enumerate()
                             .skip(start))
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

fn try_focus((i, child): (usize, &mut ListChild), source: direction::Direction)
             -> Option<usize> {
    match *child {
        ListChild::Delimiter => None,
        ListChild::Row(_, ref mut view) => {
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
            .map(ListChild::label)
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0) + 1;

        // println_stderr!("Offset: {}", offset);
        self.scrollbase.draw(printer, |printer, i| match self.children[i] {
            ListChild::Row(ref label, ref view) => {
                printer.print((0, 0), label);
                view.draw(&printer.offset((offset, 0), i == self.focus));
            }
            ListChild::Delimiter => (),
        });
    }

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        // We'll show 2 columns: the labels, and the views.
        let label_width = self.children
            .iter()
            .map(ListChild::label)
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);

        let view_size = self.children
            .iter_mut()
            .filter_map(ListChild::view)
            .map(|v| v.required_size(req).x)
            .max()
            .unwrap_or(0);

        if self.children.len() > req.y {
            Vec2::new(label_width + 1 + view_size + 2, req.y)
        } else {
            Vec2::new(label_width + 1 + view_size, self.children.len())
        }
    }

    fn layout(&mut self, size: Vec2) {
        self.scrollbase.set_heights(size.y, self.children.len());

        // We'll show 2 columns: the labels, and the views.
        let label_width = self.children
            .iter()
            .map(ListChild::label)
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);

        let spacing = 1;
        let scrollbar_width = if self.children.len() > size.y { 2 } else { 0 };

        let available = if label_width + spacing + scrollbar_width > size.x {
            // We have no space for the kids! :(
            0
        } else {
            size.x - label_width - spacing - scrollbar_width
        };

        // println_stderr!("Available: {}", available);

        for child in self.children.iter_mut().filter_map(ListChild::view) {
            child.layout(Vec2::new(available, 1));
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.children.is_empty() {
            return EventResult::Ignored;
        }

        if let ListChild::Row(_, ref mut view) = self.children[self.focus] {
            let result = view.on_event(event.clone());
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
        let i = if let Some(i) =
            self.iter_mut(rel.is_none(),
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

    fn call_on_any<'a>(&mut self, selector: &Selector,
                    mut callback: Box<FnMut(&mut Any) + 'a>) {
        for view in self.children
            .iter_mut()
            .filter_map(ListChild::view) {
            view.call_on_any(selector, Box::new(|any| callback(any)));
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<(), ()> {
        if let Some(i) = self.children
               .iter_mut()
               .enumerate()
               .filter_map(|(i, v)| v.view().map(|v| (i, v)))
               .filter_map(|(i, v)| v.focus_view(selector).ok().map(|_| i))
               .next() {
            self.focus = i;
            Ok(())
        } else {
            Err(())
        }
    }
}
