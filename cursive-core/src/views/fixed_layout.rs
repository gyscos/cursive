use crate::{
    direction::{Absolute, Direction, Relative},
    event::{AnyCb, Event, EventResult, Key},
    rect::Rect,
    view::{CannotFocus, IntoBoxedView, Selector, ViewNotFound},
    {Printer, Vec2, View, With},
};

/// Arranges its children in a fixed layout.
///
/// Usually meant to use an external layout engine.
///
/// # Examples
///
/// ```rust
/// use cursive_core::{
///     views::{Button, FixedLayout, TextView},
///     Rect,
/// };
///
/// let layout = FixedLayout::new()
///     .child(Rect::from_size((0, 0), (1, 1)), TextView::new("/"))
///     .child(Rect::from_size((14, 0), (1, 1)), TextView::new(r"\"))
///     .child(Rect::from_size((0, 2), (1, 1)), TextView::new(r"\"))
///     .child(Rect::from_size((14, 2), (1, 1)), TextView::new("/"))
///     .child(
///         Rect::from_size((3, 1), (11, 1)),
///         Button::new("Clickme", |s| s.quit()),
///     );
/// ```
pub struct FixedLayout {
    children: Vec<Child>,
    focus: usize,
}

// TODO: Add an option to have percentages as size/positions

/// Represents a child view inside the FixedLayout.
struct Child {
    view: Box<dyn View>,
    position: Rect,
}

impl Child {
    // Convenient function to look for a focusable child in an iterator.
    fn focuser(source: Direction) -> impl Fn((usize, &mut Self)) -> Option<(usize, EventResult)> {
        move |(i, c)| c.view.take_focus(source).ok().map(|res| (i, res))
    }
}

new_default!(FixedLayout);

impl FixedLayout {
    /// Returns a new, empty `FixedLayout`.
    pub fn new() -> Self {
        FixedLayout {
            children: Vec::new(),
            focus: 0,
        }
    }

    /// Adds a child. Chainable variant.
    #[must_use]
    pub fn child<V: IntoBoxedView>(self, position: Rect, view: V) -> Self {
        self.with(|s| s.add_child(position, view))
    }

    /// Adds a child.
    pub fn add_child<V: IntoBoxedView>(&mut self, position: Rect, view: V) {
        self.children.push(Child {
            view: view.into_boxed_view(),
            position,
        });
    }

    /// Returns index of focused inner view
    pub fn get_focus_index(&self) -> usize {
        self.focus
    }

    /// Attempts to set the focus on the given child.
    ///
    /// Returns `Err(())` if `index >= self.len()`, or if the view at the
    /// given index does not accept focus.
    pub fn set_focus_index(&mut self, index: usize) -> Result<EventResult, ViewNotFound> {
        self.children
            .get_mut(index)
            .and_then(|child| child.view.take_focus(Direction::none()).ok())
            .map(|res| self.set_focus_unchecked(index).and(res))
            .ok_or(ViewNotFound)
    }

    fn set_focus_unchecked(&mut self, index: usize) -> EventResult {
        if index != self.focus {
            let result = self.children[self.focus].view.on_event(Event::FocusLost);
            self.focus = index;
            result
        } else {
            EventResult::Consumed(None)
        }
    }

    /// How many children are in this view.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if this view has no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns a reference to a child.
    pub fn get_child(&self, i: usize) -> Option<&dyn View> {
        self.children.get(i).map(|c| &*c.view)
    }

    /// Returns a mutable reference to a child.
    pub fn get_child_mut(&mut self, i: usize) -> Option<&mut dyn View> {
        self.children.get_mut(i).map(|c| &mut *c.view)
    }

    /// Sets the position for the given child.
    pub fn set_child_position(&mut self, i: usize, position: Rect) {
        self.children[i].position = position;
    }

    /// Removes a child.
    ///
    /// If `i` is within bounds, the removed child will be returned.
    pub fn remove_child(&mut self, i: usize) -> Option<Box<dyn View>> {
        if i >= self.len() {
            return None;
        }

        if self.focus > i || (self.focus != 0 && self.focus == self.children.len() - 1) {
            self.focus -= 1;
        }

        Some(self.children.remove(i).view)
    }

    /// Looks for the child containing a view with the given name.
    ///
    /// Returns `Some(i)` if `self.get_child(i)` has the given name, or
    /// contains a view with the given name.
    ///
    /// Returns `None` if the given name was not found.
    pub fn find_child_from_name(&mut self, name: &str) -> Option<usize> {
        let selector = Selector::Name(name);
        for (i, c) in self.children.iter_mut().enumerate() {
            let mut found = false;
            c.view.call_on_any(&selector, &mut |_| found = true);
            if found {
                return Some(i);
            }
        }
        None
    }

    fn iter_mut<'a>(
        source: Direction,
        children: &'a mut [Child],
    ) -> Box<dyn Iterator<Item = (usize, &mut Child)> + 'a> {
        let children = children.iter_mut().enumerate();
        match source {
            Direction::Rel(Relative::Front) => Box::new(children),
            Direction::Rel(Relative::Back) => Box::new(children.rev()),
            Direction::Abs(abs) => {
                // Sort children by the given direction
                let mut children: Vec<_> = children.collect();
                children.sort_by_key(|(_, c)| c.position.edge(abs));
                Box::new(children.into_iter())
            }
        }
    }

    fn circular_mut(
        start: usize,
        children: &mut [Child],
    ) -> impl Iterator<Item = (usize, &mut Child)> {
        let (head, tail) = children.split_at_mut(start);

        let head = head.iter_mut().enumerate();
        let tail = tail
            .iter_mut()
            .enumerate()
            .map(move |(i, c)| (i + start, c));

        tail.chain(head)
    }

    fn move_focus_rel(&mut self, target: Relative) -> EventResult {
        let source = Direction::Rel(target.swap());
        let focus_res = Self::iter_mut(source, &mut self.children)
            .skip(self.focus + 1)
            .find_map(Child::focuser(source));

        if let Some((i, res)) = focus_res {
            return self.set_focus_unchecked(i).and(res);
        }

        EventResult::Ignored
    }

    fn move_focus_abs(&mut self, target: Absolute) -> EventResult {
        let source = Direction::Abs(target.opposite());
        let (orientation, rel) = target.split();

        fn intersects(a: (usize, usize), b: (usize, usize)) -> bool {
            a.1 >= b.0 && a.0 <= b.1
        }

        let current_position = self.children[self.focus].position;
        let current_side = current_position.side(orientation.swap());
        let current_edge = current_position.edge(target);

        let focus_res = Self::iter_mut(source, &mut self.children)
            .filter(|(_, c)| {
                // Only select children actually aligned with us
                Some(rel) == Relative::a_to_b(current_edge, c.position.edge(target))
                    && intersects(c.position.side(orientation.swap()), current_side)
            })
            .find_map(Child::focuser(source));

        if let Some((i, res)) = focus_res {
            return self.set_focus_unchecked(i).and(res);
        }
        EventResult::Ignored
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

            let position = match position.checked_sub(offset) {
                None => return None,
                Some(pos) => pos,
            };

            if let Some((i, res)) = self
                .children
                .iter_mut()
                .enumerate()
                .filter(|(_, c)| c.position.contains(position))
                .find_map(Child::focuser(Direction::none()))
            {
                return Some(self.set_focus_unchecked(i).and(res));
            }
        }

        None
    }
}

impl View for FixedLayout {
    fn draw(&self, printer: &Printer) {
        for (i, child) in self.children.iter().enumerate() {
            child
                .view
                .draw(&printer.windowed(child.position).focused(i == self.focus));
        }
    }

    fn layout(&mut self, _size: Vec2) {
        // TODO: re-compute children positions?
        for child in &mut self.children {
            child.view.layout(child.position.size());
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.is_empty() {
            return EventResult::Ignored;
        }

        let res = self
            .check_focus_grab(&event)
            .unwrap_or(EventResult::Ignored);

        let child = &mut self.children[self.focus];

        let result = child
            .view
            .on_event(event.relativized(child.position.top_left()));

        res.and(match result {
            EventResult::Ignored => match event {
                Event::Shift(Key::Tab) => self.move_focus_rel(Relative::Front),
                Event::Key(Key::Tab) => self.move_focus_rel(Relative::Back),
                Event::Key(Key::Left) => self.move_focus_abs(Absolute::Left),
                Event::Key(Key::Right) => self.move_focus_abs(Absolute::Right),
                Event::Key(Key::Up) => self.move_focus_abs(Absolute::Up),
                Event::Key(Key::Down) => self.move_focus_abs(Absolute::Down),
                _ => EventResult::Ignored,
            },
            res => res,
        })
    }

    fn important_area(&self, size: Vec2) -> Rect {
        if self.is_empty() {
            return Rect::from_size((0, 0), size);
        }

        let child = &self.children[self.focus];

        child.view.important_area(child.position.size()) + child.position.top_left()
    }

    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        self.children
            .iter()
            .map(|c| c.position.bottom_right() + (1, 1))
            .fold(Vec2::zero(), Vec2::max)
    }

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        match source {
            Direction::Abs(Absolute::None) => {
                // We want to guarantee:
                // * If the current focus _is_ focusable, keep it
                // * If it isn't, find _any_ focusable view, and focus it
                // * Otherwise, we can't take focus.
                let focus_res = Self::circular_mut(self.focus, &mut self.children)
                    .find_map(Child::focuser(source));
                if let Some((i, res)) = focus_res {
                    return Ok(self.set_focus_unchecked(i).and(res));
                }

                Err(CannotFocus)
            }
            source => {
                let focus_res =
                    Self::iter_mut(source, &mut self.children).find_map(Child::focuser(source));
                if let Some((i, res)) = focus_res {
                    return Ok(self.set_focus_unchecked(i).and(res));
                }

                Err(CannotFocus)
            }
        }
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        for child in &mut self.children {
            child.view.call_on_any(selector, callback);
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        let focus_res = self
            .children
            .iter_mut()
            .enumerate()
            .find_map(|(i, c)| c.view.focus_view(selector).ok().map(|res| (i, res)));
        if let Some((i, res)) = focus_res {
            return Ok(self.set_focus_unchecked(i).and(res));
        }

        Err(ViewNotFound)
    }
}

// TODO: blueprints?
