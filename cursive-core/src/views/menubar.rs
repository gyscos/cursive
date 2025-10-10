use crate::{
    direction,
    event::*,
    menu,
    rect::Rect,
    style::PaletteStyle,
    utils::markup::StyledString,
    view::{CannotFocus, Position, View},
    views::{MenuPopup, OnEventView},
    Cursive, Printer, Vec2,
};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

/// Current state of the menubar
#[derive(PartialEq, Debug)]
enum State {
    /// The menubar is inactive.
    Inactive,
    /// The menubar is actively selected.
    ///
    /// It will receive input.
    Selected,
    /// The menubar is still visible, but a submenu is open.
    ///
    /// It will not receive input.
    Submenu,
}

/// Shows a single-line list of items, with pop-up menus when one is selected.
///
/// The [`Cursive`] root already includes a menubar
/// that you just need to configure.
///
/// [`Cursive`]: crate::Cursive::menubar
pub struct Menubar {
    /// Menu items in this menubar.
    root: menu::Tree,

    /// TODO: move this out of this view.
    pub autohide: bool,
    focus: usize,

    // TODO: make Menubar impl View and take out the State management
    state: State,
}

new_default!(Menubar);

impl Menubar {
    /// Creates a new, empty menubar.
    pub fn new() -> Self {
        Menubar {
            root: menu::Tree::new(),
            autohide: true,
            state: State::Inactive,
            focus: 0,
        }
    }

    /// Hides the menubar.
    fn hide(&mut self) {
        self.state = State::Inactive;
    }

    /// True if we should be receiving events.
    pub fn receive_events(&self) -> bool {
        self.state == State::Selected
    }

    /// True if some submenus are visible.
    pub fn has_submenu(&self) -> bool {
        self.state == State::Submenu
    }

    /// Returns `true` if we should be drawn.
    pub fn visible(&self) -> bool {
        !self.autohide || self.state != State::Inactive
    }

    /// Adds a new item to the menubar.
    pub fn insert(&mut self, i: usize, item: menu::Item) -> &mut Self {
        self.root.insert(i, item);
        self
    }

    /// Adds a new item to the menubar.
    pub fn item(&mut self, item: menu::Item) -> &mut Self {
        let i = self.root.len();
        self.insert(i, item)
    }

    /// Adds a new item to the menubar.
    ///
    /// The item will use the given title, and on selection, will open a
    /// popup-menu with the given menu tree.
    pub fn add_subtree<S>(&mut self, title: S, menu: menu::Tree) -> &mut Self
    where
        S: Into<StyledString>,
    {
        let i = self.root.len();
        self.insert_subtree(i, title, menu)
    }

    /// Adds a delimiter to the menubar.
    pub fn add_delimiter(&mut self) -> &mut Self {
        let i = self.root.len();
        self.insert_delimiter(i)
    }

    /// Adds a leaf node to the menubar.
    pub fn add_leaf<S, F>(&mut self, title: S, cb: F) -> &mut Self
    where
        S: Into<StyledString>,
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        let i = self.root.len();
        self.insert_leaf(i, title, cb)
    }

    /// Insert a new item at the given position.
    pub fn insert_subtree<S>(&mut self, i: usize, title: S, menu: menu::Tree) -> &mut Self
    where
        S: Into<StyledString>,
    {
        self.root.insert_subtree(i, title, menu);
        self
    }

    /// Inserts a new delimiter at the given position.
    ///
    /// It will show up as `|`.
    pub fn insert_delimiter(&mut self, i: usize) -> &mut Self {
        self.root.insert_delimiter(i);
        self
    }

    /// Inserts a new leaf node at the given position.
    ///
    /// It will be directly actionable.
    pub fn insert_leaf<S, F>(&mut self, i: usize, title: S, cb: F) -> &mut Self
    where
        S: Into<StyledString>,
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        self.root.insert_leaf(i, title, cb);
        self
    }

    /// Removes all menu items from this menubar.
    pub fn clear(&mut self) {
        self.root.clear();
        self.focus = 0;
    }

    /// Returns the number of items in this menubar.
    pub fn len(&self) -> usize {
        self.root.len()
    }

    /// Returns `true` if this menubar is empty.
    pub fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    /// Returns the item at the given position.
    ///
    /// Returns `None` if `i > self.len()`
    pub fn get_subtree(&mut self, i: usize) -> Option<&mut menu::Tree> {
        self.root.get_subtree(i)
    }

    /// Looks for an item with the given label.
    pub fn find_subtree(&mut self, label: &str) -> Option<&mut menu::Tree> {
        self.root.find_subtree(label)
    }

    /// Returns the position of the item with the given label.
    ///
    /// Returns `None` if no such label was found.
    pub fn find_position(&mut self, label: &str) -> Option<usize> {
        self.root.find_position(label)
    }

    /// Remove the item at the given position.
    pub fn remove(&mut self, i: usize) {
        self.root.remove(i);
    }

    fn child_at(&self, x: usize) -> Option<usize> {
        if x == 0 {
            return None;
        }
        let mut offset = 1;

        for (i, child) in self.root.children.iter().enumerate() {
            offset += child.label().width() + 2;
            if x < offset {
                return Some(i);
            }
        }

        None
    }

    fn select_child(&mut self, open_only: bool) -> EventResult {
        match self.root.children[self.focus] {
            menu::Item::Leaf { ref cb, .. } if !open_only => {
                // Go inactive after an action.
                self.state = State::Inactive;
                EventResult::Consumed(Some(cb.clone()))
            }
            menu::Item::Subtree { ref tree, .. } => {
                // First, we need a new Arc to send the callback,
                // since we don't know when it will be called.
                let menu = Arc::clone(tree);

                self.state = State::Submenu;
                let offset = Vec2::new(
                    self.root.children[..self.focus]
                        .iter()
                        .map(|child| child.label().width() + 2)
                        .sum(),
                    usize::from(self.autohide),
                );
                // Since the closure will be called multiple times,
                // we also need a new Arc on every call.
                EventResult::with_cb(move |s| show_child(s, offset, Arc::clone(&menu)))
            }
            _ => EventResult::Ignored,
        }
    }
}

fn show_child(s: &mut Cursive, offset: Vec2, menu: Arc<menu::Tree>) {
    // Adds a new layer located near the item title with the menu popup.
    // Also adds two key callbacks on this new view, to handle `left` and
    // `right` key presses.
    // (If the view itself listens for a `left` or `right` press, it will
    // consume it before our OnEventView. This means sub-menus can properly
    // be entered.)
    s.screen_mut().add_layer_at(
        Position::absolute(offset),
        OnEventView::new(
            MenuPopup::new(menu)
                .on_dismiss(Cursive::select_menubar)
                .on_action(|s| s.menubar().state = State::Inactive),
        )
        .on_event(Key::Right, |s| {
            s.pop_layer();
            s.select_menubar();
            // Act as if we sent "Right" then "Down"
            s.menubar().on_event(Event::Key(Key::Right)).process(s);
            if let EventResult::Consumed(Some(cb)) = s.menubar().on_event(Event::Key(Key::Down)) {
                cb(s);
            }
        })
        .on_event(Key::Left, |s| {
            s.pop_layer();
            s.select_menubar();
            // Act as if we sent "Left" then "Down"
            s.menubar().on_event(Event::Key(Key::Left)).process(s);
            if let EventResult::Consumed(Some(cb)) = s.menubar().on_event(Event::Key(Key::Down)) {
                cb(s);
            }
        }),
    );
}

impl View for Menubar {
    fn draw(&self, printer: &Printer) {
        // Draw the bar at the top
        printer.with_style(PaletteStyle::View, |printer| {
            printer.print_hline((0, 0), printer.size.x, " ");

            // TODO: draw the rest
            let mut offset = 1;
            for (i, item) in self.root.children.iter().enumerate() {
                let label = item.styled_label();
                let label_width = label.width();
                // We print disabled items differently, except delimiters,
                // which are still white.
                let enabled = printer.enabled && (item.is_enabled() || item.is_delimiter());

                // We don't want to show HighlightInactive when we're not selected,
                // because it's ugly on the menubar.
                let selected = (self.state != State::Inactive) && (i == self.focus);

                let style = match (enabled, selected) {
                    (false, _) => PaletteStyle::Secondary,
                    (true, true) => PaletteStyle::Highlight,
                    _ => PaletteStyle::Primary,
                };

                printer.with_style(style, |printer| {
                    printer.print((offset, 0), " ");
                    offset += 1;
                    printer.print_styled((offset, 0), label);
                    offset += label_width;
                    printer.print((offset, 0), " ");
                    offset += 1;
                });
            }
        });
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Esc) => {
                self.hide();
                return EventResult::with_cb(Cursive::clear);
            }
            Event::Key(Key::Left) => loop {
                // TODO: fix endless loop if nothing is enabled?
                if self.focus > 0 {
                    self.focus -= 1;
                } else {
                    self.focus = self.root.len() - 1;
                }
                if self.root.children[self.focus].is_enabled() {
                    break;
                }
            },
            Event::Key(Key::Right) => loop {
                if self.focus + 1 < self.root.len() {
                    self.focus += 1;
                } else {
                    self.focus = 0;
                }
                if self.root.children[self.focus].is_enabled() {
                    break;
                }
            },
            Event::Key(Key::Down) => {
                return self.select_child(true);
            }
            Event::Key(Key::Enter) => {
                return self.select_child(false);
            }
            Event::Mouse {
                event: MouseEvent::Press(btn),
                position,
                offset,
            } if position.fits(offset) && position.y == offset.y => {
                if let Some(child) = position
                    .checked_sub(offset)
                    .and_then(|pos| self.child_at(pos.x))
                {
                    if self.root.children[child].is_enabled() {
                        self.focus = child;
                        if btn == MouseButton::Left {
                            return self.select_child(true);
                        }
                    }
                }
            }
            Event::Mouse {
                event: MouseEvent::Release(btn),
                position,
                offset,
            } if position.fits(offset) && position.y == offset.y => {
                if let Some(child) = position
                    .checked_sub(offset)
                    .and_then(|pos| self.child_at(pos.x))
                {
                    if self.focus == child
                        && btn == MouseButton::Left
                        && self.root.children[child].is_leaf()
                    {
                        return self.select_child(false);
                    }
                }
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                ..
            } => {
                self.hide();
                return EventResult::with_cb(Cursive::clear);
            }
            _ => return EventResult::Ignored,
        }
        EventResult::Consumed(None)
    }

    fn take_focus(&mut self, _: direction::Direction) -> Result<EventResult, CannotFocus> {
        self.state = State::Selected;
        Ok(EventResult::consumed())
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        // TODO: scroll the options if the screen is too small?

        // We add 2 to the length of every label for marin.
        // Also, we add 1 at the beginning.
        // (See the `draw()` method)
        let width = self
            .root
            .children
            .iter()
            .map(|item| item.label().len() + 2)
            .sum();

        Vec2::new(width, 1)
    }

    fn important_area(&self, size: Vec2) -> Rect {
        if self.root.is_empty() {
            return Rect::from_size(Vec2::zero(), size);
        }

        // X position is 1 (margin before the first item) + sum of widths
        // And each item has a 2 cells padding.
        let x = 1 + self.root.children[..self.focus]
            .iter()
            .map(|child| child.label().width() + 2)
            .sum::<usize>();

        let width = self.root.children[self.focus].label().width();

        Rect::from_size((x, 0), (width, 1))
    }
}
