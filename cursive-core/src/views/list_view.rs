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
    /// The focused row was handed out mutably (see [`ListView::row_mut`]).
    ///
    /// The next event or focus request checks that the child at `focus` is still a row that
    /// takes focus. If it is, nothing changes. If it is not, this becomes `Lost`.
    Unchecked,
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

    // Set when a scan found no focusable child, so events don't repeat it.
    //
    // Cleared whenever the children may have changed.
    scan_failed: bool,

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
            scan_failed: false,
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
    /// The list cannot see what is done with the child. If this is the focused one, the list
    /// checks it on the next event or focus request: if it is not a row that takes focus
    /// anymore (it was replaced by a delimiter, or by a view that refuses focus), the focus is
    /// moved like when the focused child is removed, and `on_select` is called for the new
    /// selection.
    ///
    /// The check asks the focused view to take focus again, so a view that was replaced by
    /// another one that takes focus gets it, and the selection does not change. The list cannot
    /// tell this from a change to the same view, which is then asked to take focus again too
    /// (which is harmless for the usual views).
    ///
    /// # Panics
    ///
    /// Panics if `id >= self.len()`.
    pub fn row_mut(&mut self, id: usize) -> &mut ListChild {
        self.row_will_change(id);
        &mut self.children[id]
    }

    /// Gives mutable access to the child at the given position.
    ///
    /// Returns `None` if `id >= self.len()`.
    ///
    /// See [`ListView::row_mut`] for what happens to the focus.
    pub fn try_row_mut(&mut self, id: usize) -> Option<&mut ListChild> {
        self.row_will_change(id);
        self.children.get_mut(id)
    }

    // The child at `id` is about to be handed out mutably.
    fn row_will_change(&mut self, id: usize) {
        self.scan_failed = false;
        if id == self.focus && self.focus_state == FocusState::Settled {
            self.focus_state = FocusState::Unchecked;
        }
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
        self.scan_failed = false;
        self.focus = self.focus.min(self.len().saturating_sub(1));
        self.lose_focus();
    }

    /// Adds a view to the end of the list.
    pub fn add_child<V: IntoBoxedView + 'static>(&mut self, label: impl Into<String>, view: V) {
        let view = view.into_boxed_view();

        // Why were we doing this here?
        // view.take_focus(direction::Direction::none());
        self.scan_failed = false;
        self.children.push(ListChild::Row(label.into(), view));
        self.children_heights.push(0);
    }

    /// Attempts to set the focus to the given position.
    ///
    /// Returns `None` if `if >= self.len()` or if the child at this location is not focusable.
    ///
    /// If the previously focused child was removed or replaced, this also
    /// calls the `on_select` callback for the new selection.
    pub fn set_focus(&mut self, id: usize) -> Option<EventResult> {
        let checked = self.recheck_focus();
        let lost = self.focus_state == FocusState::Lost;
        let res = self.set_focus_quiet(id)?;
        Some(checked.and(self.notify_if(lost, res)))
    }

    // Like `set_focus`, but never calls `on_select`.
    fn set_focus_quiet(&mut self, id: usize) -> Option<EventResult> {
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
        self.scan_failed = false;
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
        self.scan_failed = false;
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
        self.scan_failed = false;
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
        self.scan_failed = false;
        res
    }

    // The focused child is gone: the next focus acquisition is a selection change.
    fn lose_focus(&mut self) {
        if matches!(
            self.focus_state,
            FocusState::Settled | FocusState::Unchecked
        ) {
            self.focus_state = FocusState::Lost;
        }
    }

    // Checks the focused child if it was handed out with `row_mut`.
    //
    // If it is still a row that takes focus, it has it again (which does not change the
    // selection). If it is not, the focus is lost: the child is told, and the next focus
    // acquisition is a selection change.
    //
    // Returns what the child answered. Does nothing in any other state.
    fn recheck_focus(&mut self) -> EventResult {
        if self.focus_state != FocusState::Unchecked {
            return EventResult::Ignored;
        }
        match self.set_focus_quiet(self.focus) {
            // Nothing happened, as far as the caller is concerned.
            Some(EventResult::Consumed(None)) => EventResult::Ignored,
            Some(res) => res,
            None => {
                let res = self.unfocus_child();
                self.focus_state = FocusState::Lost;
                res
            }
        }
    }

    // Focus a child again after the focused one was removed or replaced.
    //
    // Returns the new child's focus result, followed by `on_select` if a
    // previous selection was lost.
    fn repair_focus(&mut self) -> Result<EventResult, CannotFocus> {
        // (If no child can take focus, what the old child answered when losing it is dropped.)
        let checked = self.recheck_focus();
        if self.focus_state == FocusState::Settled {
            return Ok(checked);
        }
        let notify = self.focus_state == FocusState::Lost;
        for index in (self.focus..self.len()).chain((0..self.focus).rev()) {
            if let Some(res) = self.set_focus_quiet(index) {
                return Ok(checked.and(self.notify_if(notify, res)));
            }
        }
        self.scan_failed = true;
        Err(CannotFocus)
    }

    // Appends the `on_select` callback to `res` if `notify` is set.
    //
    // `notify` must have been read before the focus was moved, since moving
    // it settles the state.
    fn notify_if(&self, notify: bool, res: EventResult) -> EventResult {
        if notify {
            res.and(self.on_select_result())
        } else {
            res
        }
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

            let checked = self.recheck_focus();
            let lost = self.focus_state == FocusState::Lost;

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
                    let res = self.set_focus_unchecked(i).and(res);
                    return Some(checked.and(self.notify_if(lost, res)));
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
        if event == Event::FocusLost {
            match self.focus_state {
                FocusState::Settled => (),
                // The focused row may have been replaced, but it is the one to tell.
                FocusState::Unchecked => self.focus_state = FocusState::Settled,
                FocusState::Unset | FocusState::Lost => return EventResult::Ignored,
            }
        }
        let res = self
            .check_focus_grab(&event)
            .unwrap_or(EventResult::Ignored);
        let res = if self.focus_state != FocusState::Settled && !self.scan_failed {
            match self.repair_focus() {
                Ok(repaired) => res.and(repaired),
                Err(_) => res,
            }
        } else {
            res
        };
        // Nothing can take focus: only mouse events (scrolling, clicks on
        // rows that react without focus) still go to the child at `focus`.
        if self.focus_state != FocusState::Settled && !matches!(event, Event::Mouse { .. }) {
            return res;
        }

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
        let checked = self.recheck_focus();
        let lost = self.focus_state == FocusState::Lost;
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
        let res = self.set_focus_unchecked(i).and(res);
        Ok(checked.and(self.notify_if(lost, res)))
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        self.scan_failed = false;
        for view in self.children.iter_mut().filter_map(ListChild::view) {
            view.call_on_any(selector, callback);
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        // Try to focus each view. Skip over delimiters.
        let checked = self.recheck_focus();
        let lost = self.focus_state == FocusState::Lost;
        if let Some((i, res)) = self
            .children
            .iter_mut()
            .enumerate()
            .filter_map(|(i, v)| v.view().map(|v| (i, v)))
            .find_map(|(i, v)| v.focus_view(selector).ok().map(|res| (i, res)))
        {
            let res = self.set_focus_unchecked(i).and(res);
            Ok(checked.and(self.notify_if(lost, res)))
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
    use crate::event::{MouseButton, MouseEvent};
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

    // A row that refuses focus, counts `take_focus` calls and logs mouse events.
    struct Stubborn {
        scans: Arc<Mutex<usize>>,
        log: Arc<Mutex<Vec<String>>>,
    }
    impl View for Stubborn {
        fn draw(&self, _: &Printer) {}
        fn take_focus(&mut self, _: direction::Direction) -> Result<EventResult, CannotFocus> {
            *self.scans.lock().unwrap() += 1;
            Err(CannotFocus)
        }
        fn on_event(&mut self, event: Event) -> EventResult {
            if matches!(event, Event::Mouse { .. }) {
                self.log.lock().unwrap().push("mouse".into());
                return EventResult::consumed();
            }
            EventResult::Ignored
        }
    }
    fn mouse(event: MouseEvent) -> Event {
        Event::Mouse {
            event,
            position: Vec2::new(0, 0),
            offset: Vec2::zero(),
        }
    }

    fn stubborn_list(rows: usize) -> (ListView, Arc<Mutex<usize>>, Arc<Mutex<Vec<String>>>) {
        let scans = Arc::new(Mutex::new(0));
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut list = ListView::new();
        for i in 0..rows {
            list.add_child(
                i.to_string(),
                Stubborn {
                    scans: scans.clone(),
                    log: log.clone(),
                },
            );
        }
        (list, scans, log)
    }

    #[test]
    fn unfocusable_rows_still_get_mouse_events() {
        let (mut list, _, log) = stubborn_list(3);
        for _ in 0..4 {
            assert!(list.on_event(mouse(MouseEvent::WheelDown)).is_consumed());
        }
        assert!(!list.on_event(Event::Char('x')).is_consumed());
        assert_eq!(log.lock().unwrap().len(), 4);
    }

    #[test]
    fn unfocusable_rows_are_not_rescanned_on_every_event() {
        let (mut list, scans, _) = stubborn_list(3);
        for _ in 0..4 {
            list.on_event(Event::Char('x'));
            list.on_event(mouse(MouseEvent::WheelDown));
        }
        // One scan of the 3 rows, not one per event.
        assert_eq!(*scans.lock().unwrap(), 3);

        // Changing the children allows a new scan.
        list.add_delimiter();
        list.on_event(Event::Char('x'));
        assert_eq!(*scans.lock().unwrap(), 6);
        list.take_focus(direction::Direction::none()).unwrap_err();
        assert_eq!(*scans.lock().unwrap(), 9);
    }

    #[test]
    fn every_way_of_regaining_focus_selects_once_after_removal() {
        type Regain = fn(&mut ListView) -> Option<EventResult>;
        let ways: [(&str, Regain); 5] = [
            ("none", |l| l.take_focus(direction::Direction::none()).ok()),
            ("front", |l| {
                l.take_focus(direction::Direction::front()).ok()
            }),
            ("back", |l| l.take_focus(direction::Direction::back()).ok()),
            ("click", |l| {
                // Rows are 1 high, the label column is 1 wide.
                l.layout(Vec2::new(20, 5));
                Some(l.on_event(Event::Mouse {
                    event: MouseEvent::Press(MouseButton::Left),
                    position: Vec2::new(5, 0),
                    offset: Vec2::zero(),
                }))
            }),
            ("set_focus", |l| l.set_focus(0)),
        ];
        for (name, regain) in ways {
            let (mut list, log) = list();
            process(list.set_focus(1).unwrap());
            list.remove_child(1);
            log.lock().unwrap().clear();
            process(regain(&mut list).unwrap());
            let selected = format!("select {}", list.children[list.focus].label());
            let selects = |log: &Arc<Mutex<Vec<String>>>| {
                log.lock()
                    .unwrap()
                    .iter()
                    .filter(|e| e.starts_with("select"))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            assert_eq!(selects(&log), vec![selected], "{name}");
            exercise(&mut list);
            assert_eq!(selects(&log).len(), 1, "{name}: second select");
        }
    }

    // Replaces the focused row (1) of `list()` with `replace`, then regains the focus in every
    // way. The selection changes once, to the row that gets the focus.
    fn check_replacing_the_focused_row(
        name: &str,
        replace: impl Fn(&mut ListView, &Arc<Mutex<Vec<String>>>),
    ) {
        type Regain = fn(&mut ListView) -> Option<EventResult>;
        // The row that gets the focus: events and a request for no direction prefer the following
        // row, like after a removal.
        let ways: [(&str, Regain, usize); 6] = [
            ("event", |l| Some(l.on_event(Event::Char('x'))), 2),
            (
                "none",
                |l| l.take_focus(direction::Direction::none()).ok(),
                2,
            ),
            (
                "front",
                |l| l.take_focus(direction::Direction::front()).ok(),
                0,
            ),
            (
                "back",
                |l| l.take_focus(direction::Direction::back()).ok(),
                2,
            ),
            (
                "click",
                |l| {
                    // Rows are 1 high, the label column is 1 wide.
                    l.layout(Vec2::new(20, 5));
                    Some(l.on_event(Event::Mouse {
                        event: MouseEvent::Press(MouseButton::Left),
                        position: Vec2::new(5, 0),
                        offset: Vec2::zero(),
                    }))
                },
                0,
            ),
            ("set_focus", |l| l.set_focus(0), 0),
        ];
        for (way, regain, focus) in ways {
            let (mut list, log) = list();
            process(list.set_focus(1).unwrap());
            replace(&mut list, &log);
            log.lock().unwrap().clear();
            process(regain(&mut list).unwrap());
            exercise(&mut list);
            let selects: Vec<String> = log
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.starts_with("select"))
                .cloned()
                .collect();
            assert_eq!(list.focus, focus, "{name} / {way}");
            assert_eq!(selects, vec![format!("select {focus}")], "{name} / {way}");
            assert_eq!(list.focus_state, FocusState::Settled, "{name} / {way}");
        }
    }

    #[test]
    fn replacing_the_focused_row_with_a_delimiter_moves_the_focus() {
        // The repair prefers the following row, like after a removal.
        check_replacing_the_focused_row("row_mut", |list, _| {
            *list.row_mut(1) = ListChild::Delimiter
        });
        check_replacing_the_focused_row("try_row_mut", |list, _| {
            *list.try_row_mut(1).unwrap() = ListChild::Delimiter
        });
    }

    #[test]
    fn replacing_the_focused_row_with_a_row_that_refuses_focus_moves_the_focus() {
        check_replacing_the_focused_row("refusing", |list, log| {
            *list.row_mut(1) = ListChild::Row(
                "refuses".into(),
                Box::new(Probe {
                    id: 9,
                    accepts: false,
                    log: log.clone(),
                }),
            )
        });
    }

    #[test]
    fn replacing_the_focused_row_with_a_focusable_one_keeps_the_selection() {
        let (mut list, log) = list();
        process(list.set_focus(1).unwrap());
        *list.row_mut(1) = ListChild::Row(
            "new".into(),
            Box::new(Probe {
                id: 9,
                accepts: true,
                log: log.clone(),
            }),
        );
        log.lock().unwrap().clear();
        exercise(&mut list);
        exercise(&mut list);
        assert_eq!(list.focus, 1);
        // The new view was given the focus once, and nothing was selected.
        let log = log.lock().unwrap();
        assert_eq!(log.iter().filter(|e| *e == "focus 9").count(), 1);
        assert_eq!(log.iter().filter(|e| *e == "callback 9").count(), 1);
        assert!(!log.iter().any(|e| e.starts_with("select")));
    }

    #[test]
    fn replacing_the_only_focusable_row_leaves_a_pending_selection() {
        let (mut list, log) = list();
        list.remove_child(2);
        list.remove_child(0);
        process(list.set_focus(0).unwrap());
        *list.row_mut(0) = ListChild::Delimiter;
        exercise(&mut list);
        exercise(&mut list);
        assert_eq!(list.focus_state, FocusState::Lost);
        assert!(list.scan_failed);

        // A row that can take focus is selected when it is added.
        list.add_child(
            "late",
            Probe {
                id: 7,
                accepts: true,
                log: log.clone(),
            },
        );
        log.lock().unwrap().clear();
        exercise(&mut list);
        assert_eq!(list.focus, 1);
        assert!(log.lock().unwrap().contains(&"select late".to_string()));
    }

    #[test]
    fn changing_another_row_does_not_check_the_focus() {
        let (mut list, log) = list();
        process(list.set_focus(0).unwrap());
        *list.row_mut(1) = ListChild::Delimiter;
        assert_eq!(list.focus_state, FocusState::Settled);
        log.lock().unwrap().clear();
        exercise(&mut list);
        assert!(
            !log.lock()
                .unwrap()
                .iter()
                .any(|e| e.starts_with("focus") || e.starts_with("select"))
        );
    }

    #[test]
    fn editing_the_focused_row_in_place_does_not_swallow_events() {
        struct Plain;
        impl View for Plain {
            fn draw(&self, _: &Printer) {}
            fn take_focus(&mut self, _: direction::Direction) -> Result<EventResult, CannotFocus> {
                Ok(EventResult::Consumed(None))
            }
        }
        let mut list = ListView::new().child("a", Plain).child("b", Plain);
        list.take_focus(direction::Direction::none()).unwrap();
        list.row_mut(0);
        // Nothing wants this event, and the check changed nothing.
        assert!(!list.on_event(Event::Char('x')).is_consumed());
        assert_eq!(list.focus(), 0);
        assert_eq!(list.focus_state, FocusState::Settled);
    }

    #[test]
    fn losing_focus_reaches_a_row_that_was_handed_out() {
        let (mut list, log) = list();
        process(list.set_focus(1).unwrap());
        list.row_mut(1);
        log.lock().unwrap().clear();
        process(list.on_event(Event::FocusLost));
        assert_eq!(*log.lock().unwrap(), vec!["event 1 FocusLost"]);
    }
}

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use crate::event::{MouseButton, MouseEvent};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Clone)]
    struct Flags {
        accepts: Arc<AtomicBool>,
        focused: Arc<AtomicBool>,
        events: Arc<Mutex<Vec<String>>>,
        id: usize,
    }
    struct P(Flags);
    impl View for P {
        fn draw(&self, _: &Printer) {}
        fn take_focus(&mut self, _: direction::Direction) -> Result<EventResult, CannotFocus> {
            if !self.0.accepts.load(Ordering::SeqCst) {
                return Err(CannotFocus);
            }
            self.0.focused.store(true, Ordering::SeqCst);
            Ok(EventResult::Consumed(None))
        }
        fn on_event(&mut self, event: Event) -> EventResult {
            if event == Event::FocusLost {
                self.0.focused.store(false, Ordering::SeqCst);
            }
            self.0
                .events
                .lock()
                .unwrap()
                .push(format!("{} {:?}", self.0.id, event));
            EventResult::Ignored
        }
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self, n: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) as usize) % n
        }
    }

    fn row(next_id: &mut usize, flags: &mut Vec<Flags>, accepts: bool) -> ListChild {
        let f = Flags {
            accepts: Arc::new(AtomicBool::new(accepts)),
            focused: Arc::new(AtomicBool::new(false)),
            events: Arc::new(Mutex::new(Vec::new())),
            id: *next_id,
        };
        *next_id += 1;
        flags.push(f.clone());
        ListChild::Row(format!("r{}", f.id), Box::new(P(f)))
    }

    #[test]
    fn random_operations_keep_the_focus_consistent() {
        let selected = Arc::new(Mutex::new(Vec::<String>::new()));
        let mut siv = crate::Cursive::new();
        for seed in 0..400u64 {
            let mut rng = Rng(seed * 7919 + 1);
            let mut next_id = 0;
            let mut all_flags: Vec<Flags> = Vec::new();
            let sel = selected.clone();
            let mut list =
                ListView::new().on_select(move |_, l| sel.lock().unwrap().push(l.clone()));
            let mut trace: Vec<String> = Vec::new();
            for step in 0..40 {
                selected.lock().unwrap().clear();
                let op = rng.next(12);
                let lost_before = list.focus_state == FocusState::Lost;
                let mut res = EventResult::Ignored;
                let mut was_event = false;
                match op {
                    0 => {
                        let acc = rng.next(4) != 0;
                        let c = row(&mut next_id, &mut all_flags, acc);
                        if let ListChild::Row(l, v) = c {
                            list.add_child(l.clone(), v);
                            trace.push(format!("add {l} acc={acc}"));
                        }
                    }
                    1 => {
                        list.add_delimiter();
                        trace.push("add delim".into());
                    }
                    2 if !list.is_empty() => {
                        let i = rng.next(list.len());
                        list.remove_child(i);
                        trace.push(format!("remove {i}"));
                    }
                    3 => {
                        let n = rng.next(4);
                        let mut v = Vec::new();
                        for _ in 0..n {
                            if rng.next(4) == 0 {
                                v.push(ListChild::Delimiter);
                            } else {
                                let acc = rng.next(4) != 0;
                                v.push(row(&mut next_id, &mut all_flags, acc));
                            }
                        }
                        trace.push(format!("set_children {n}"));
                        list.set_children(v);
                    }
                    4 => {
                        list.clear();
                        trace.push("clear".into());
                    }
                    5 => {
                        let d = [
                            direction::Direction::none(),
                            direction::Direction::front(),
                            direction::Direction::back(),
                            direction::Direction::up(),
                            direction::Direction::down(),
                        ][rng.next(5)];
                        trace.push(format!("take_focus {d:?}"));
                        if let Ok(r) = list.take_focus(d) {
                            res = r;
                        }
                    }
                    6 => {
                        let e = [
                            Event::Key(Key::Up),
                            Event::Key(Key::Down),
                            Event::Key(Key::Tab),
                            Event::Shift(Key::Tab),
                            Event::Char('x'),
                        ][rng.next(5)]
                        .clone();
                        trace.push(format!("event {e:?}"));
                        list.layout(Vec2::new(20, 10));
                        res = list.on_event(e);
                        was_event = true;
                    }
                    7 if !list.is_empty() => {
                        list.layout(Vec2::new(20, 10));
                        let y = rng.next(list.len() + 1);
                        trace.push(format!("click y={y}"));
                        res = list.on_event(Event::Mouse {
                            event: MouseEvent::Press(MouseButton::Left),
                            position: Vec2::new(5, y),
                            offset: Vec2::zero(),
                        });
                        was_event = true;
                    }
                    8 if !list.is_empty() => {
                        let i = rng.next(list.len());
                        if let ListChild::Row(_, _) = list.get_row(i) {
                            let lbl = list.get_row(i).label().to_string();
                            let id: usize = lbl[1..].parse().unwrap();
                            let f = &all_flags[id];
                            let now = !f.accepts.load(Ordering::SeqCst);
                            f.accepts.store(now, Ordering::SeqCst);
                            list.row_mut(i);
                            trace.push(format!("toggle {i} -> accepts={now}"));
                        }
                    }
                    9 if !list.is_empty() => {
                        let i = rng.next(list.len());
                        trace.push(format!("set_focus {i}"));
                        if let Some(r) = list.set_focus(i) {
                            res = r;
                        }
                    }
                    10 if !list.is_empty() => {
                        // Replace any row, the focused one included.
                        let i = rng.next(list.len());
                        let new = match rng.next(3) {
                            0 => ListChild::Delimiter,
                            1 => row(&mut next_id, &mut all_flags, true),
                            _ => row(&mut next_id, &mut all_flags, false),
                        };
                        *list.row_mut(i) = new;
                        trace.push(format!("replace {i}"));
                    }
                    _ => {}
                }
                res.process(&mut siv);
                let sels = selected.lock().unwrap().clone();
                let ctx =
                    || format!("seed {seed} step {step}\ntrace: {trace:#?}\nselected {sels:?}");
                // Invariants.
                if !list.is_empty() {
                    assert!(list.focus < list.len(), "focus out of range\n{}", ctx());
                }
                if list.focus_state == FocusState::Settled && !list.is_empty() {
                    match &list.children[list.focus] {
                        ListChild::Delimiter => {
                            panic!("settled on a delimiter\n{}", ctx())
                        }
                        ListChild::Row(l, _) => {
                            let id: usize = l[1..].parse().unwrap();
                            assert!(
                                all_flags[id].focused.load(Ordering::SeqCst),
                                "settled on a row that is not focused ({l})\n{}",
                                ctx()
                            );
                        }
                    }
                }
                let focused_now: Vec<_> = list
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ListChild::Row(l, _) => {
                            let id: usize = l[1..].parse().unwrap();
                            all_flags[id]
                                .focused
                                .load(Ordering::SeqCst)
                                .then(|| l.clone())
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    focused_now.len() <= 1,
                    "several focused rows {focused_now:?}\n{}",
                    ctx()
                );
                if sels.len() > 1 && !(lost_before && sels.len() == 2 && was_event) {
                    panic!("on_select fired {} times\n{}", sels.len(), ctx());
                }
                if let Some(s) = sels.last() {
                    if list.focus_state == FocusState::Settled {
                        assert_eq!(
                            s,
                            list.children[list.focus].label(),
                            "selected label is not the focused row\n{}",
                            ctx()
                        );
                    }
                }
                if was_event && !list.is_empty() {
                    let any = list.children.iter().any(|c| match c {
                        ListChild::Row(l, _) => {
                            let id: usize = l[1..].parse().unwrap();
                            all_flags[id].accepts.load(Ordering::SeqCst)
                        }
                        _ => false,
                    });
                    if any {
                        assert!(
                            list.focus_state == FocusState::Settled,
                            "an event left the list without focus although a row accepts it\n{}",
                            ctx()
                        );
                    }
                }
            }
        }
    }
}
