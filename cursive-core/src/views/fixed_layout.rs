use crate::direction::{Absolute, Direction, Relative};
use crate::event::{Event, EventResult, Key};
use crate::rect::Rect;
use crate::view::IntoBoxedView;
use crate::{Printer, Vec2, View, With};

/// Arranges its children in a fixed layout.
///
/// Usually meant to use an external layout engine.
///
/// # Examples
///
/// ```rust
/// use cursive_core::{
///     views::{FixedLayout, TextView, Button},
///     Rect,
/// };
///
/// let layout = FixedLayout::new()
///     .child(Rect::from_size((0,0), (1,1)), TextView::new("/"))
///     .child(Rect::from_size((14,0), (1,1)), TextView::new(r"\"))
///     .child(Rect::from_size((0,2), (1,1)), TextView::new(r"\"))
///     .child(Rect::from_size((14,2), (1,1)), TextView::new("/"))
///     .child(Rect::from_size((3,1), (11,1)), Button::new("Clickme", |s| s.quit()));
/// ````
pub struct FixedLayout {
    children: Vec<Child>,
    focus: usize,
}

/// Represents a child view inside the FixedLayout.
struct Child {
    view: Box<dyn View>,
    position: Rect,
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
    pub fn child<V: IntoBoxedView>(self, position: Rect, view: V) -> Self {
        self.with(|s| s.add_child(position, view))
    }

    /// Adds a child.
    pub fn add_child<V: IntoBoxedView>(&mut self, position: Rect, view: V) {
        self.children.push(Child {
            view: view.as_boxed_view(),
            position,
        });
    }

    /// Returns index of focused inner view
    pub fn get_focus_index(&self) -> usize {
        self.focus
    }

    /// Attemps to set the focus on the given child.
    ///
    /// Returns `Err(())` if `index >= self.len()`, or if the view at the
    /// given index does not accept focus.
    pub fn set_focus_index(&mut self, index: usize) -> Result<(), ()> {
        if self
            .children
            .get_mut(index)
            .map(|child| child.view.take_focus(Direction::none()))
            .unwrap_or(false)
        {
            self.focus = index;
            Ok(())
        } else {
            Err(())
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

        if self.focus > i
            || (self.focus != 0 && self.focus == self.children.len() - 1)
        {
            self.focus -= 1;
        }

        Some(self.children.remove(i).view)
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

    fn circular_mut<'a>(
        start: usize,
        children: &'a mut [Child],
    ) -> impl Iterator<Item = (usize, &mut Child)> + 'a {
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
        for (i, c) in
            Self::iter_mut(source, &mut self.children).skip(self.focus + 1)
        {
            if c.view.take_focus(source) {
                self.focus = i;
                return EventResult::Consumed(None);
            }
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

        let children =
            Self::iter_mut(source, &mut self.children).filter(|(_, c)| {
                // Only select children actually aligned with us
                Some(rel)
                    == Relative::a_to_b(current_edge, c.position.edge(target))
                    && intersects(
                        c.position.side(orientation.swap()),
                        current_side,
                    )
            });

        for (i, c) in children {
            if c.view.take_focus(source) {
                self.focus = i;
                return EventResult::Consumed(None);
            }
        }

        EventResult::Ignored
    }

    fn check_focus_grab(&mut self, event: &Event) {
        if let Event::Mouse {
            offset,
            position,
            event,
        } = *event
        {
            if !event.grabs_focus() {
                return;
            }

            let position = match position.checked_sub(offset) {
                None => return,
                Some(pos) => pos,
            };

            for (i, child) in self.children.iter_mut().enumerate() {
                if child.position.contains(position)
                    && child.view.take_focus(Direction::none())
                {
                    self.focus = i;
                }
            }
        }
    }
}

impl View for FixedLayout {
    fn draw(&self, printer: &Printer) {
        for child in &self.children {
            child.view.draw(&printer.windowed(child.position));
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

        self.check_focus_grab(&event);

        let child = &mut self.children[self.focus];

        let result = child
            .view
            .on_event(event.relativized(child.position.top_left()));

        match result {
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
        }
    }

    fn important_area(&self, size: Vec2) -> Rect {
        if self.is_empty() {
            return Rect::from_size((0, 0), size);
        }

        let child = &self.children[self.focus];

        child.view.important_area(child.position.size())
            + child.position.top_left()
    }

    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        self.children
            .iter()
            .map(|c| c.position.bottom_left() + (1, 1))
            .fold(Vec2::zero(), Vec2::max)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        match source {
            Direction::Abs(Absolute::None) => {
                // We want to guarantee:
                // * If the current focus _is_ focusable, keep it
                // * If it isn't, find _any_ focusable view, and focus it
                // * Otherwise, we can't take focus.
                for (i, c) in
                    Self::circular_mut(self.focus, &mut self.children)
                {
                    if c.view.take_focus(source) {
                        self.focus = i;
                        return true;
                    }
                }

                false
            }
            source => {
                for (i, c) in Self::iter_mut(source, &mut self.children) {
                    if c.view.take_focus(source) {
                        self.focus = i;
                        return true;
                    }
                }

                false
            }
        }
    }
}
