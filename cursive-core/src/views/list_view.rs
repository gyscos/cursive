use crate::builder::{Config, Context, Error, Resolvable};
use crate::{
    Cursive, Printer, Vec2, With, direction,
    event::{AnyCb, Callback, Event, EventResult, Key},
    rect::Rect,
    view::{CannotFocus, IntoBoxedView, Selector, View, ViewNotFound},
};
use log::debug;
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

/// Represents a child from a [`ListView`].
pub enum ListChild {
    /// A single row, with a label and a view.
    Row(String, Box<dyn View>),
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

    fn view(&mut self) -> Option<&mut dyn View> {
        match *self {
            ListChild::Row(_, ref mut view) => Some(view.as_mut()),
            _ => None,
        }
    }
}

type ListCallback = dyn Fn(&mut Cursive, &String) + Send + Sync;

/// Whether `ListView::focus` points at a child that currently has focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusState {
    /// The child at `focus` has focus.
    Settled,
    /// No child was focused yet (new or cleared list).
    ///
    /// Focus is acquired on the next event, without calling `on_select`.
    Unset,
    /// The focused child was removed or replaced.
    ///
    /// Focus is acquired on the next event, and `on_select` is called.
    Lost,
}

/// Displays a list of elements.
pub struct ListView {
    children: Vec<ListChild>,

    // Height for each child.
    // This should have the same size as the `children` list.
    children_heights: Vec<usize>,

    // Which child is focused? Should index into the `children` list.
    //
    // This is `0` when `children` is empty.
    focus: usize,

    // Whether the child at `focus` actually has focus.
    focus_state: FocusState,

    // This callback is called when the selection is changed.
    on_select: Option<Arc<ListCallback>>,
}

// Implement `Default` around `ListView::new`
new_default!(ListView);

impl ListView {
    /// Creates a new, empty `ListView`.
    pub fn new() -> Self {
        ListView {
            children: Vec::new(),
            children_heights: Vec::new(),
            focus: 0,
            focus_state: FocusState::Unset,
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
    ///
    /// # Panics
    ///
    /// If `id >= self.len()`.
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

    /// Gives mutable access to the child at the given position.
    ///
    /// Returns `None` if `id >= self.len()`.
    pub fn try_row_mut(&mut self, id: usize) -> Option<&mut ListChild> {
        self.children.get_mut(id)
    }

    /// Gives access to the child at the given position.
    ///
    /// Returns `None` if `id >= self.len()`.
    pub fn try_row(&self, id: usize) -> Option<&ListChild> {
        self.children.get(id)
    }

    /// Sets the children for this view.
    ///
    /// Clamps the selection to the new length. Focus is acquired on the next
    /// event or explicit focus request, preserving any returned callbacks and
    /// calling the `on_select` callback if a child was previously focused.
    pub fn set_children(&mut self, children: Vec<ListChild>) {
        self.children_heights.resize(children.len(), 0);
        self.children = children;
        self.focus = self.focus.min(self.len().saturating_sub(1));
        self.lose_focus();
    }

    /// Adds a view to the end of the list.
    pub fn add_child<V: IntoBoxedView + 'static>(&mut self, label: impl Into<String>, view: V) {
        let view = view.into_boxed_view();

        // Why were we doing this here?
        // view.take_focus(direction::Direction::none());
        self.children.push(ListChild::Row(label.into(), view));
        self.children_heights.push(0);
    }

    /// Attempts to set the focus to the given position.
    ///
    /// Returns `None` if `if >= self.len()` or if the child at this location is not focusable.
    pub fn set_focus(&mut self, id: usize) -> Option<EventResult> {
        if id >= self.len() {
            return None;
        }

        let ListChild::Row(_, ref mut view) = self.children[id] else {
            return None;
        };

        let Ok(res) = view.take_focus(direction::Direction::none()) else {
            return None;
        };

        Some(self.set_focus_unchecked(id).and(res))
    }

    /// Removes all children from this view.
    pub fn clear(&mut self) {
        self.children.clear();
        self.children_heights.clear();
        self.focus = 0;
        self.focus_state = FocusState::Unset;
    }

    /// Adds a view to the end of the list.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn child<V: IntoBoxedView + 'static>(self, label: &str, view: V) -> Self {
        self.with(|s| s.add_child(label, view))
    }

    /// Adds a delimiter to the end of the list.
    pub fn add_delimiter(&mut self) {
        self.children.push(ListChild::Delimiter);
        self.children_heights.push(0);
    }

    /// Adds a delimiter to the end of the list.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn delimiter(self) -> Self {
        self.with(Self::add_delimiter)
    }

    /// Removes a child from the view.
    ///
    /// Preserves the focused child when possible. If it is removed, focus is
    /// acquired on the next event or explicit focus request, preferring a
    /// following child and then a preceding child, and the `on_select`
    /// callback is called for the new selection.
    ///
    /// # Panics
    ///
    /// If `index >= self.len()`.
    pub fn remove_child(&mut self, index: usize) -> ListChild {
        self.children_heights.remove(index);
        let child = self.children.remove(index);
        if index == self.focus {
            self.lose_focus();
        } else if index < self.focus {
            self.focus -= 1;
        }
        self.focus = self.focus.min(self.len().saturating_sub(1));
        child
    }

    /// Removes a child from the view.
    ///
    /// Returns `None` if `index >= self.len()`.
    pub fn try_remove(&mut self, index: usize) -> Option<ListChild> {
        if index >= self.len() {
            None
        } else {
            Some(self.remove_child(index))
        }
    }

    /// Sets a callback to be used when an item is selected.
    #[crate::callback_helpers]
    pub fn set_on_select<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive, &String) + 'static + Send + Sync,
    {
        self.on_select = Some(Arc::new(cb));
    }

    /// Sets a callback to be used when an item is selected.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_select<F>(self, cb: F) -> Self
    where
        F: Fn(&mut Cursive, &String) + 'static + Send + Sync,
    {
        self.with(|s| s.set_on_select(cb))
    }

    /// Returns the index of the currently focused item.
    ///
    /// Returns `0` if the list is empty.
    pub fn focus(&self) -> usize {
        self.focus
    }

    fn iter_mut<'a>(
        &'a mut self,
        from_focus: bool,
        source: direction::Relative,
    ) -> Box<dyn Iterator<Item = (usize, &'a mut ListChild)> + 'a> {
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

    fn unfocus_child(&mut self) -> EventResult {
        self.children
            .get_mut(self.focus)
            .and_then(ListChild::view)
            .map(|v| v.on_event(Event::FocusLost))
            .unwrap_or(EventResult::Ignored)
    }

    // Move focus to the given index, regardless of whether that child accepts focus.
    fn set_focus_unchecked(&mut self, index: usize) -> EventResult {
        let res = if index != self.focus && self.focus_state == FocusState::Settled {
            self.unfocus_child()
        } else {
            EventResult::Consumed(None)
        };
        self.focus = index;
        self.focus_state = FocusState::Settled;
        res
    }

    // The focused child is gone: the next focus acquisition is a selection change.
    fn lose_focus(&mut self) {
        if self.focus_state == FocusState::Settled {
            self.focus_state = FocusState::Lost;
        }
    }

    // Focus a child again after the focused one was removed or replaced.
    //
    // Returns the new child's focus result, followed by `on_select` if a
    // previous selection was lost.
    fn repair_focus(&mut self) -> Result<EventResult, CannotFocus> {
        let notify = self.focus_state == FocusState::Lost;
        for index in (self.focus..self.len()).chain((0..self.focus).rev()) {
            if let Some(res) = self.set_focus(index) {
                return Ok(if notify {
                    res.and(self.on_select_result())
                } else {
                    res
                });
            }
        }
        Err(CannotFocus)
    }

    // Runs the `on_select` callback (if any) with the focused child's label.
    fn on_select_result(&self) -> EventResult {
        EventResult::Consumed(self.on_select.clone().map(|cb| {
            let label = String::from(self.children[self.focus].label());
            Callback::from_fn(move |s| cb(s, &label))
        }))
    }

    fn move_focus(&mut self, n: usize, source: direction::Direction) -> EventResult {
        let (i, res) = if let Some((i, res)) = source
            .relative(direction::Orientation::Vertical)
            .and_then(|rel| {
                // The iterator starts at the focused element.
                // We don't want that one.
                self.iter_mut(true, rel)
                    .skip(1)
                    .filter_map(|p| try_focus(p, source))
                    .take(n)
                    .last()
            }) {
            (i, res)
        } else {
            return EventResult::Ignored;
        };
        let res = res.and(self.set_focus_unchecked(i));

        res.and(self.on_select_result())
    }

    fn labels_width(&self) -> usize {
        self.children
            .iter()
            .map(ListChild::label)
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0)
    }

    fn check_focus_grab(&mut self, event: &Event) -> Option<EventResult> {
        if let Event::Mouse {
            offset,
            position,
            event,
        } = *event
        {
            if !event.grabs_focus() {
                return None;
            }

            let mut position = position.checked_sub(offset)?;

            // eprintln!("Rel pos: {:?}", position);

            // Now that we have a relative position, checks for buttons?
            for (i, (child, &height)) in self
                .children
                .iter_mut()
                .zip(&self.children_heights)
                .enumerate()
            {
                // Keep trying to remove the child height to the position
                if let Some(y) = position.y.checked_sub(height) {
                    // Not this child. Move on.
                    position.y = y;
                    continue;
                }

                // We found the correct target, try to focus it.
                if let ListChild::Row(_, view) = child
                    && let Ok(res) = view.take_focus(direction::Direction::none())
                {
                    return Some(self.set_focus_unchecked(i).and(res));
                }
                // We found the target, but we can't focus it.
                break;
            }
        }
        None
    }
}

fn try_focus(
    (i, child): (usize, &mut ListChild),
    source: direction::Direction,
) -> Option<(usize, EventResult)> {
    match *child {
        ListChild::Delimiter => None,
        ListChild::Row(_, ref mut view) => match view.take_focus(source) {
            Ok(res) => Some((i, res)),
            Err(CannotFocus) => None,
        },
    }
}

impl View for ListView {
    fn layout_key(&self) -> u64 {
        self.children.iter().fold(
            crate::view::layout_key_seed::<Self>(),
            |key, child| match child {
                ListChild::Delimiter => crate::view::combine_layout_key(key, &0u8),
                ListChild::Row(label, view) => {
                    crate::view::combine_layout_key(key, &(1u8, label.width(), view.layout_key()))
                }
            },
        )
    }

    fn draw(&self, printer: &Printer) {
        if self.children.is_empty() {
            return;
        }

        let offset = self.labels_width() + 1;
        let mut y = 0;

        // Only rows at least partly visible need drawing.
        let visible = printer.visible_range(direction::Orientation::Vertical);

        debug!("Offset: {}", offset);
        for (i, (child, &height)) in self.children.iter().zip(&self.children_heights).enumerate() {
            if y >= visible.end {
                break;
            }
            if y + height <= visible.start {
                y += height;
                continue;
            }
            match child {
                ListChild::Row(label, view) => {
                    printer.print((0, y), label);
                    view.draw(
                        &printer
                            .offset((offset, y))
                            .cropped((printer.size.x, height))
                            .focused(i == self.focus),
                    );
                }
                ListChild::Delimiter => (), // TODO: draw delimiters?
            }
            y += height;
        }
    }

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        // We'll show 2 columns: the labels, and the views.
        let label_width = self
            .children
            .iter()
            .map(ListChild::label)
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);

        let view_size =
            direction::Orientation::Vertical.stack(self.children.iter_mut().map(|c| match c {
                ListChild::Delimiter => Vec2::new(0, 1),
                ListChild::Row(_, view) => view.required_size(req),
            }));

        view_size + (1 + label_width, 0)
    }

    fn layout(&mut self, size: Vec2) {
        // We'll show 2 columns: the labels, and the views.
        let label_width = self
            .children
            .iter()
            .map(ListChild::label)
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);

        let spacing = 1;

        let available = size.x.saturating_sub(label_width + spacing);

        debug!("Available: {}", available);

        self.children_heights.resize(self.children.len(), 0);

        for (child, height) in self.children.iter_mut().zip(&mut self.children_heights) {
            match child.view() {
                Some(child) => {
                    *height = child.required_size(size).y;
                    child.layout(Vec2::new(available, *height));
                }
                None => {
                    // Delimiters have height=1
                    *height = 1;
                }
            }
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.children.is_empty() {
            return EventResult::Ignored;
        }

        // Losing focus must not acquire a new child just to unfocus it.
        if self.focus_state != FocusState::Settled && event == Event::FocusLost {
            return EventResult::Ignored;
        }
        let res = self
            .check_focus_grab(&event)
            .unwrap_or(EventResult::Ignored);
        let res = if self.focus_state != FocusState::Settled {
            match self.repair_focus() {
                Ok(repaired) => res.and(repaired),
                Err(_) => return res,
            }
        } else {
            res
        };

        // Send the event to the focused child.
        let labels_width = self.labels_width();
        if let ListChild::Row(_, ref mut view) = self.children[self.focus] {
            let y = self.children_heights[..self.focus].iter().sum();
            let offset = (labels_width + 1, y);
            let result = view.on_event(event.relativized(offset));
            if result.is_consumed() {
                return res.and(result);
            }
        }

        // If the child ignored this event, change the focus.
        res.and(match event {
            Event::Key(Key::Up) if self.focus > 0 => {
                self.move_focus(1, direction::Direction::down())
            }
            Event::Key(Key::Down) if self.focus + 1 < self.children.len() => {
                self.move_focus(1, direction::Direction::up())
            }
            Event::Key(Key::PageUp) => self.move_focus(10, direction::Direction::down()),
            Event::Key(Key::PageDown) => self.move_focus(10, direction::Direction::up()),
            Event::Key(Key::Home) | Event::Ctrl(Key::Home) => {
                self.move_focus(usize::MAX, direction::Direction::back())
            }
            Event::Key(Key::End) | Event::Ctrl(Key::End) => {
                self.move_focus(usize::MAX, direction::Direction::front())
            }
            Event::Key(Key::Tab) => self.move_focus(1, direction::Direction::front()),
            Event::Shift(Key::Tab) => self.move_focus(1, direction::Direction::back()),
            _ => EventResult::Ignored,
        })
    }

    fn take_focus(&mut self, source: direction::Direction) -> Result<EventResult, CannotFocus> {
        if self.focus_state != FocusState::Settled && source == direction::Direction::none() {
            return self.repair_focus();
        }
        let rel = source.relative(direction::Orientation::Vertical);
        let (i, res) = if let Some((i, res)) = self
            .iter_mut(rel.is_none(), rel.unwrap_or(direction::Relative::Front))
            .find_map(|p| try_focus(p, source))
        {
            (i, res)
        } else {
            // No one wants to be in focus
            return Err(CannotFocus);
        };
        Ok(self.set_focus_unchecked(i).and(res))
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        for view in self.children.iter_mut().filter_map(ListChild::view) {
            view.call_on_any(selector, callback);
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        // Try to focus each view. Skip over delimiters.
        if let Some((i, res)) = self
            .children
            .iter_mut()
            .enumerate()
            .filter_map(|(i, v)| v.view().map(|v| (i, v)))
            .find_map(|(i, v)| v.focus_view(selector).ok().map(|res| (i, res)))
        {
            Ok(self.set_focus_unchecked(i).and(res))
        } else {
            Err(ViewNotFound)
        }
    }

    fn important_area(&self, size: Vec2) -> Rect {
        if self.children.is_empty() {
            return Rect::from_size(Vec2::zero(), size);
        }

        let labels_width = self.labels_width();

        // This is the size of the focused view
        let area = match self.children[self.focus] {
            ListChild::Row(_, ref view) => {
                let available = Vec2::new(size.x.saturating_sub(labels_width + 1), 1);
                view.important_area(available) + (labels_width, 0)
            }
            ListChild::Delimiter => Rect::from_size((0, 0), (size.x, 1)),
        };

        // This is how far down the focused view is.
        // (The size of everything above.)
        let y_offset: usize = self.children_heights[..self.focus].iter().copied().sum();

        area + (0, y_offset)
    }
}

impl Resolvable for ListChild {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        if config.as_str() == Some("delimiter") {
            Ok(ListChild::Delimiter)
        } else {
            let view = context.resolve(&config["view"])?;
            let label = context.resolve(&config["label"])?;
            Ok(ListChild::Row(label, view))
        }
    }
}

#[crate::blueprint(ListView::new())]
struct Blueprint {
    children: Vec<ListChild>,

    on_select: Option<_>,
}

#[cfg(test)]
mod focus_tests {
    use super::*;
    use std::sync::Mutex;

    struct Probe {
        id: usize,
        accepts: bool,
        log: Arc<Mutex<Vec<String>>>,
    }
    impl View for Probe {
        fn draw(&self, _: &Printer) {}
        fn take_focus(&mut self, _: direction::Direction) -> Result<EventResult, CannotFocus> {
            if !self.accepts {
                return Err(CannotFocus);
            }
            self.log.lock().unwrap().push(format!("focus {}", self.id));
            let log = self.log.clone();
            let id = self.id;
            Ok(EventResult::with_cb(move |_| {
                log.lock().unwrap().push(format!("callback {id}"))
            }))
        }
        fn on_event(&mut self, event: Event) -> EventResult {
            self.log
                .lock()
                .unwrap()
                .push(format!("event {} {event:?}", self.id));
            EventResult::Ignored
        }
    }
    fn list() -> (ListView, Arc<Mutex<Vec<String>>>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let select_log = log.clone();
        let mut list = ListView::new()
            .on_select(move |_, label| select_log.lock().unwrap().push(format!("select {label}")));
        for id in 0..3 {
            list.add_child(
                id.to_string(),
                Probe {
                    id,
                    accepts: true,
                    log: log.clone(),
                },
            );
        }
        (list, log)
    }
    fn process(res: EventResult) {
        res.process(&mut crate::Cursive::new());
    }
    fn exercise(list: &mut ListView) {
        let size = Vec2::new(20, 5);
        list.important_area(size);
        list.layout(size);
        let mut buffer = crate::buffer::PrintBuffer::new();
        buffer.resize(size);
        let buffer = parking_lot::RwLock::new(buffer);
        list.draw(&Printer::new(
            size,
            &crate::theme::Theme::default(),
            &buffer,
        ));
        process(list.on_event(Event::Char('x')));
    }

    #[test]
    fn removal_preserves_surviving_focus() {
        for removed in [0, 2] {
            let (mut list, log) = list();
            process(list.set_focus(1).unwrap());
            log.lock().unwrap().clear();
            list.remove_child(removed);
            assert_eq!(list.children[list.focus].label(), "1");
            exercise(&mut list);
            assert!(
                log.lock()
                    .unwrap()
                    .iter()
                    .all(|entry| !entry.starts_with("focus")
                        && !entry.starts_with("select")
                        && !entry.contains("FocusLost"))
            );
        }
    }

    #[test]
    fn removed_focus_is_repaired_once_with_callbacks() {
        let (mut list, log) = list();
        process(list.set_focus(1).unwrap());
        log.lock().unwrap().clear();
        list.remove_child(1);
        assert!(log.lock().unwrap().is_empty());
        exercise(&mut list);
        exercise(&mut list);
        let log = log.lock().unwrap();
        assert_eq!(log.iter().filter(|entry| *entry == "focus 2").count(), 1);
        assert_eq!(log.iter().filter(|entry| *entry == "callback 2").count(), 1);
        assert!(!log.iter().any(|entry| entry.contains("FocusLost")));
    }

    #[test]
    fn replacement_skips_unfocusable_children_and_falls_back() {
        let (mut list, log) = list();
        process(list.set_focus(2).unwrap());
        list.set_children(vec![
            ListChild::Row(
                "accept".into(),
                Box::new(Probe {
                    id: 4,
                    accepts: true,
                    log: log.clone(),
                }),
            ),
            ListChild::Row(
                "reject".into(),
                Box::new(Probe {
                    id: 5,
                    accepts: false,
                    log: log.clone(),
                }),
            ),
            ListChild::Delimiter,
        ]);
        exercise(&mut list);
        assert_eq!(list.focus, 0);
        list.set_children(vec![ListChild::Delimiter]);
        exercise(&mut list);
        assert!(list.take_focus(direction::Direction::none()).is_err());
        list.set_children(vec![]);
        exercise(&mut list);
        assert_eq!(list.focus, 0);
    }

    #[test]
    fn removing_last_focused_row_repairs_before_event_and_geometry() {
        let (mut list, log) = list();
        process(list.set_focus(2).unwrap());
        log.lock().unwrap().clear();
        list.remove_child(2);
        exercise(&mut list);
        assert_eq!(list.focus, 1);
        assert!(log.lock().unwrap().contains(&"callback 1".to_string()));
    }

    #[test]
    fn pending_focus_lost_does_not_focus_replacement() {
        let (mut list, log) = list();
        process(list.set_focus(1).unwrap());
        list.remove_child(1);
        log.lock().unwrap().clear();
        process(list.on_event(Event::FocusLost));
        assert!(log.lock().unwrap().is_empty());
        process(list.take_focus(direction::Direction::none()).unwrap());
        assert_eq!(list.focus, 1);
        assert_eq!(
            *log.lock().unwrap(),
            vec!["focus 2", "callback 2", "select 2"]
        );
    }

    #[test]
    fn repair_after_removal_selects_after_child_callback() {
        let (mut list, log) = list();
        process(list.set_focus(1).unwrap());
        list.remove_child(1);
        log.lock().unwrap().clear();
        exercise(&mut list);
        exercise(&mut list);
        let log = log.lock().unwrap();
        let select = log.iter().position(|entry| entry == "select 2").unwrap();
        let callback = log.iter().position(|entry| entry == "callback 2").unwrap();
        assert!(callback < select);
        assert_eq!(log.iter().filter(|entry| *entry == "select 2").count(), 1);
    }

    #[test]
    fn initial_and_cleared_focus_do_not_select() {
        let (mut list, log) = list();
        exercise(&mut list);
        process(list.take_focus(direction::Direction::none()).unwrap());
        list.clear();
        list.add_child(
            "new",
            Probe {
                id: 4,
                accepts: true,
                log: log.clone(),
            },
        );
        exercise(&mut list);
        assert!(
            !log.lock()
                .unwrap()
                .iter()
                .any(|entry| entry.starts_with("select"))
        );
    }

    #[test]
    fn replacing_focused_children_selects_new_child() {
        let (mut list, log) = list();
        process(list.set_focus(1).unwrap());
        list.set_children(vec![ListChild::Row(
            "replacement".into(),
            Box::new(Probe {
                id: 4,
                accepts: true,
                log: log.clone(),
            }),
        )]);
        log.lock().unwrap().clear();
        process(list.take_focus(direction::Direction::none()).unwrap());
        assert_eq!(
            *log.lock().unwrap(),
            vec!["focus 4", "callback 4", "select replacement"]
        );
    }

    #[test]
    fn last_removal_and_explicit_focus_clear_pending_repair() {
        let (mut list, log) = list();
        process(list.set_focus(2).unwrap());
        list.try_remove(2).unwrap();
        assert_eq!(list.focus, 1);
        process(list.set_focus(0).unwrap());
        log.lock().unwrap().clear();
        exercise(&mut list);
        assert!(
            !log.lock()
                .unwrap()
                .iter()
                .any(|entry| entry.starts_with("focus"))
        );
        list.clear();
        list.add_child(
            "new",
            Probe {
                id: 4,
                accepts: true,
                log: log.clone(),
            },
        );
        process(list.take_focus(direction::Direction::none()).unwrap());
        list.remove_child(0);
        exercise(&mut list);
        assert_eq!(list.focus, 0);
        assert!(list.try_remove(0).is_none());
    }
}
