use crate::{
    Cursive, Printer, Vec2, direction,
    event::*,
    menu,
    rect::Rect,
    style::PaletteStyle,
    utils::markup::StyledString,
    view::{CannotFocus, Position, View},
    views::{MenuPopup, OnEventView},
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

    /// Whether the user has ever selected an item (the menubar was activated).
    ///
    /// Until then `focus` is only the default starting position, not a
    /// selection to preserve when items are inserted.
    touched: bool,

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
            touched: false,
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
        // Keep an existing selection on the same item when inserting before it.
        // An untouched menubar has no selection yet: its default position
        // stays on the leftmost item.
        if self.touched && self.len() > 1 && i <= self.focus {
            self.focus += 1;
        }
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
        self.insert(i, menu::Item::subtree(title, menu))
    }

    /// Inserts a new delimiter at the given position.
    ///
    /// It will show up as `|`.
    pub fn insert_delimiter(&mut self, i: usize) -> &mut Self {
        self.insert(i, menu::Item::Delimiter)
    }

    /// Inserts a new leaf node at the given position.
    ///
    /// It will be directly actionable.
    pub fn insert_leaf<S, F>(&mut self, i: usize, title: S, cb: F) -> &mut Self
    where
        S: Into<StyledString>,
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        self.insert(i, menu::Item::leaf(title, cb))
    }

    /// Removes all menu items from this menubar.
    pub fn clear(&mut self) {
        self.root.clear();
        self.focus = 0;
        self.touched = false;
        self.hide();
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
    ///
    /// Preserves the selected item when possible; otherwise selects the next
    /// enabled item, falling back to a preceding item.
    pub fn remove(&mut self, i: usize) {
        self.root.remove(i);
        if !self.touched {
            // Nothing was ever selected: the default position stays on the leftmost item, and
            // the first activation picks the first enabled one from there.
            self.focus = 0;
            if !self.root.children.iter().any(|child| child.is_enabled()) {
                self.hide();
            }
            return;
        }
        if i < self.focus {
            self.focus -= 1;
        }
        self.focus = self.focus.min(self.len().saturating_sub(1));
        if let Some(index) = (self.focus..self.len())
            .chain((0..self.focus).rev())
            .find(|&index| self.root.children[index].is_enabled())
        {
            self.focus = index;
        } else {
            self.hide();
        }
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
        let Some(item) = self
            .root
            .children
            .get(self.focus)
            .filter(|item| item.is_enabled())
        else {
            return EventResult::Ignored;
        };
        match *item {
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
    fn layout_key(&self) -> u64 {
        self.root
            .children
            .iter()
            .fold(crate::view::layout_key_seed::<Self>(), |key, item| {
                crate::view::combine_layout_key(key, &item.label().len())
            })
    }

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
            Event::Key(key @ (Key::Left | Key::Right)) => {
                let mut index = self.focus;
                let mut found = None;
                for _ in 0..self.len() {
                    index = if key == Key::Left {
                        if index == 0 {
                            self.len() - 1
                        } else {
                            index - 1
                        }
                    } else if index + 1 == self.len() {
                        0
                    } else {
                        index + 1
                    };
                    if self.root.children[index].is_enabled() {
                        found = Some(index);
                        break;
                    }
                }
                let Some(index) = found else {
                    return EventResult::Ignored;
                };
                self.focus = index;
            }
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
                    && self.root.children[child].is_enabled()
                {
                    self.focus = child;
                    if btn == MouseButton::Left {
                        return self.select_child(true);
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
                    && self.focus == child
                    && btn == MouseButton::Left
                    && self.root.children[child].is_leaf()
                {
                    return self.select_child(false);
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
        let Some(index) = (self.focus..self.len())
            .chain(0..self.focus)
            .find(|&index| self.root.children[index].is_enabled())
        else {
            self.hide();
            return Err(CannotFocus);
        };
        self.focus = index;
        self.touched = true;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(name: &str) -> menu::Item {
        menu::Item::leaf(name, |_| panic!("disabled action ran"))
    }

    #[test]
    fn unavailable_menus_ignore_navigation_and_activation() {
        for items in [
            vec![],
            vec![menu::Item::Delimiter],
            vec![leaf("off").disabled()],
        ] {
            let mut bar = Menubar::new();
            for item in items {
                bar.item(item);
            }
            assert!(bar.take_focus(direction::Direction::none()).is_err());
            for key in [Key::Left, Key::Right, Key::Enter, Key::Down] {
                assert!(!bar.on_event(Event::Key(key)).is_consumed());
            }
        }
    }

    #[test]
    fn navigation_wraps_and_mutations_preserve_selection() {
        let mut bar = Menubar::new();
        bar.add_leaf("a", |_| {})
            .add_delimiter()
            .item(leaf("off").disabled())
            .add_leaf("b", |_| {});
        bar.take_focus(direction::Direction::none()).unwrap();
        bar.on_event(Event::Key(Key::Left));
        assert_eq!(bar.focus, 3);
        bar.on_event(Event::Key(Key::Right));
        assert_eq!(bar.focus, 0);
        bar.insert_leaf(0, "new", |_| {});
        assert_eq!(bar.focus, 1);
        bar.remove(0);
        assert_eq!(bar.focus, 0);
        bar.remove(0);
        assert_eq!(bar.root.children[bar.focus].label(), "b");
        bar.remove(bar.focus);
        assert!(!bar.receive_events());
        bar.important_area(Vec2::new(20, 1));
        bar.clear();
        assert_eq!(bar.focus, 0);
        assert!(!bar.on_event(Event::Key(Key::Right)).is_consumed());
    }

    #[test]
    fn disabled_selection_cannot_activate_with_keyboard_or_mouse() {
        let mut bar = Menubar::new();
        bar.item(leaf("off").disabled());
        for event in [
            Event::Key(Key::Enter),
            Event::Key(Key::Down),
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position: Vec2::new(2, 0),
                offset: Vec2::zero(),
            },
        ] {
            bar.on_event(event).process(&mut Cursive::new());
        }
        bar.clear();
        bar.item(menu::Item::subtree("off", menu::Tree::new()).disabled());
        assert!(!bar.select_child(true).is_consumed());
        assert!(!bar.has_submenu());
    }

    #[test]
    fn popup_switching_tolerates_cleared_menu() {
        let mut siv = Cursive::new();
        siv.menubar()
            .add_subtree("menu", menu::Tree::new().leaf("action", |_| {}));
        siv.select_menubar();
        let open = siv.menubar().on_event(Event::Key(Key::Down));
        open.process(&mut siv);
        siv.menubar().clear();
        siv.on_event(Event::Key(Key::Right));
        assert!(!siv.menubar().receive_events());
    }

    #[test]
    fn insert_before_untouched_menubar_keeps_leftmost_default() {
        let mut bar = Menubar::new();
        bar.add_leaf("File", |_| {});
        bar.insert_leaf(0, "App", |_| {});
        bar.insert_leaf(0, "Zero", |_| {});
        bar.take_focus(direction::Direction::none()).unwrap();
        assert_eq!(bar.focus, 0);
        assert_eq!(bar.root.children[bar.focus].label(), "Zero");
    }

    #[test]
    fn insert_and_remove_keep_active_selection() {
        let mut bar = Menubar::new();
        bar.add_leaf("a", |_| {})
            .add_leaf("b", |_| {})
            .add_leaf("c", |_| {});
        bar.take_focus(direction::Direction::none()).unwrap();
        bar.on_event(Event::Key(Key::Right));
        assert_eq!(bar.root.children[bar.focus].label(), "b");

        // After the selection: nothing changes.
        bar.insert_leaf(3, "d", |_| {});
        assert_eq!(bar.root.children[bar.focus].label(), "b");
        // Before (and at) the selection: same item stays selected.
        bar.insert_leaf(1, "x", |_| {});
        bar.insert_leaf(0, "y", |_| {});
        assert_eq!(bar.root.children[bar.focus].label(), "b");
        // Removals mirror this.
        bar.remove(0);
        bar.remove(0);
        assert_eq!(bar.root.children[bar.focus].label(), "b");
        bar.remove(bar.len() - 1);
        assert_eq!(bar.root.children[bar.focus].label(), "b");

        // Still the same once the menubar went inactive again.
        bar.hide();
        bar.insert_leaf(0, "z", |_| {});
        assert_eq!(bar.root.children[bar.focus].label(), "b");
    }

    /// The kind of item in the model.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Kind {
        Leaf,
        Subtree,
        Delimiter,
    }

    /// One item of the model.
    #[derive(Clone, Copy, Debug)]
    struct Entry {
        /// Unique. Also used in the label (except for delimiters).
        id: u32,
        kind: Kind,
        enabled: bool,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Mode {
        Inactive,
        Selected,
        Submenu,
    }

    /// An independent model of what a menubar should do.
    ///
    /// It is a list of items, the id of the remembered item and whether the bar was ever
    /// activated. It never looks at `focus`: the remembered item is tracked by id.
    struct Model {
        entries: Vec<Entry>,
        /// The remembered item. Only tracked once the menubar was activated.
        sel: Option<u32>,
        touched: bool,
        /// An item was removed while the menubar was never activated.
        removed_untouched: bool,
        mode: Mode,
        next_id: u32,
    }

    impl Model {
        fn position(&self, id: u32) -> usize {
            self.entries.iter().position(|e| e.id == id).unwrap()
        }

        /// The item `steps` away from `from`, wrapping around.
        fn step(&self, from: usize, steps: usize, forward: bool) -> usize {
            let len = self.entries.len();
            if forward {
                (from + steps) % len
            } else {
                (from + len - steps % len) % len
            }
        }
    }

    /// A tiny deterministic generator (xorshift64*).
    struct Rng(u64);

    impl Rng {
        fn new(seed: u64) -> Self {
            // Spread the seeds, and avoid the all-zero state.
            Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
        }

        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11
        }

        /// A number in `0..n`.
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    const DIRECTIONS: [fn() -> direction::Direction; 7] = [
        direction::Direction::none,
        direction::Direction::front,
        direction::Direction::back,
        direction::Direction::left,
        direction::Direction::right,
        direction::Direction::up,
        direction::Direction::down,
    ];

    fn check(condition: bool, what: impl FnOnce() -> String) -> Result<(), String> {
        if condition { Ok(()) } else { Err(what()) }
    }

    /// Compares the menubar with the model after an operation.
    fn check_invariants(bar: &Menubar, model: &Model) -> Result<(), String> {
        let items = &bar.root.children;
        check(items.len() == model.entries.len(), || {
            format!("{} items, expected {}", items.len(), model.entries.len())
        })?;
        for (i, (item, entry)) in items.iter().zip(&model.entries).enumerate() {
            let same = match entry.kind {
                Kind::Delimiter => item.is_delimiter(),
                Kind::Leaf => item.is_leaf() && item.label() == format!("i{}", entry.id),
                Kind::Subtree => item.is_subtree() && item.label() == format!("i{}", entry.id),
            };
            check(same && item.is_enabled() == entry.enabled, || {
                format!("item {i} is {:?}, expected {entry:?}", item.label())
            })?;
        }

        check(
            bar.receive_events() == (model.mode == Mode::Selected),
            || {
                format!(
                    "receive_events is {}, mode {:?}",
                    bar.receive_events(),
                    model.mode
                )
            },
        )?;
        check(bar.has_submenu() == (model.mode == Mode::Submenu), || {
            format!(
                "has_submenu is {}, mode {:?}",
                bar.has_submenu(),
                model.mode
            )
        })?;
        check(bar.visible() == (model.mode != Mode::Inactive), || {
            format!("visible is {}, mode {:?}", bar.visible(), model.mode)
        })?;
        check(bar.touched == model.touched, || {
            format!("touched is {}, expected {}", bar.touched, model.touched)
        })?;

        if model.entries.is_empty() {
            return check(bar.focus == 0 && model.mode == Mode::Inactive, || {
                format!(
                    "empty menubar with focus {} and mode {:?}",
                    bar.focus, model.mode
                )
            });
        }
        check(bar.focus < items.len(), || {
            format!("focus {} out of range for {} items", bar.focus, items.len())
        })?;

        if model.mode != Mode::Inactive {
            let item = &items[bar.focus];
            check(!item.is_delimiter() && item.is_enabled(), || {
                format!("active on {:?}, which cannot be selected", item.label())
            })?;
        }
        if model.touched {
            // The same item stays selected, whatever is inserted or removed around it.
            let expected = model.position(model.sel.unwrap());
            check(bar.focus == expected, || {
                format!(
                    "focus {}, expected the remembered item at {expected}",
                    bar.focus
                )
            })?;
        }
        Ok(())
    }

    /// Does `ops` random operations on a menubar and on the model, and compares them.
    ///
    /// Returns a description of the first difference. The operations are appended to `log`.
    ///
    /// The first activation of a menubar must land on its leftmost enabled item. Unless
    /// `strict`, this is not checked for a menubar from which an item was removed before its
    /// first activation (see the ignored tests below).
    fn run_menubar_fuzz(
        seed: u64,
        ops: usize,
        strict: bool,
        log: &mut Vec<String>,
    ) -> Result<(), String> {
        let mut rng = Rng::new(seed);
        let mut bar = Menubar::new();
        let mut model = Model {
            entries: Vec::new(),
            sel: None,
            touched: false,
            removed_untouched: false,
            mode: Mode::Inactive,
            next_id: 0,
        };

        for _ in 0..ops {
            let len = model.entries.len();
            let roll = rng.below(100);
            if roll < 28 && len < 8 {
                // Insert a leaf, a subtree or a delimiter, sometimes disabled.
                let i = rng.below(len + 1);
                let id = model.next_id;
                model.next_id += 1;
                let (kind, enabled) = match rng.below(10) {
                    0..=3 => (Kind::Leaf, true),
                    4 => (Kind::Leaf, false),
                    5 | 6 => (Kind::Subtree, true),
                    7 => (Kind::Subtree, false),
                    _ => (Kind::Delimiter, false),
                };
                log.push(format!(
                    "insert {kind:?}{} #{id} at {i}",
                    if enabled { "" } else { " (disabled)" }
                ));
                let label = format!("i{id}");
                let item = match kind {
                    Kind::Delimiter => menu::Item::Delimiter,
                    Kind::Leaf => menu::Item::leaf(label, |_| {}),
                    Kind::Subtree => menu::Item::subtree(label, menu::Tree::new()),
                };
                bar.insert(i, if enabled { item } else { item.disabled() });
                model.entries.insert(i, Entry { id, kind, enabled });
                if model.entries.len() == 1 {
                    // The only item is at the remembered position.
                    model.sel = model.touched.then_some(id);
                }
            } else if roll < 50 {
                if len == 0 {
                    continue;
                }
                let i = rng.below(len);
                log.push(format!("remove at {i} (#{})", model.entries[i].id));
                bar.remove(i);
                let removed = model.entries.remove(i).id;
                if !model.touched {
                    model.removed_untouched = true;
                } else if model.entries.is_empty() {
                    model.sel = None;
                    model.mode = Mode::Inactive;
                } else {
                    // Keep the remembered item if it is still there and usable. Otherwise the
                    // next enabled one, then the closest enabled one before it.
                    let at = if model.sel == Some(removed) {
                        i.min(model.entries.len() - 1)
                    } else {
                        model.position(model.sel.unwrap())
                    };
                    let usable = (at..model.entries.len())
                        .chain((0..at).rev())
                        .find(|&j| model.entries[j].enabled);
                    match usable {
                        Some(j) => model.sel = Some(model.entries[j].id),
                        None => {
                            model.sel = Some(model.entries[at].id);
                            model.mode = Mode::Inactive;
                        }
                    }
                }
            } else if roll < 65 {
                let direction = DIRECTIONS[rng.below(DIRECTIONS.len())]();
                log.push(format!("take_focus {direction:?}"));
                let result = bar.take_focus(direction);
                match model.entries.iter().position(|e| e.enabled) {
                    None => {
                        check(result.is_err(), || {
                            "take_focus worked without enabled items".into()
                        })?;
                        model.mode = Mode::Inactive;
                    }
                    Some(first) => {
                        check(result.is_ok(), || {
                            "take_focus failed with enabled items".into()
                        })?;
                        let landing = if model.touched {
                            // The remembered item, or the next enabled one after it.
                            let from = model.position(model.sel.unwrap());
                            (0..model.entries.len())
                                .map(|k| model.step(from, k, true))
                                .find(|&j| model.entries[j].enabled)
                                .unwrap()
                        } else {
                            // The first activation: the leftmost enabled item, whatever was
                            // inserted before.
                            if strict || !model.removed_untouched {
                                check(bar.focus == first, || {
                                    format!(
                                        "first activation on item {}, the leftmost enabled is {first}",
                                        bar.focus
                                    )
                                })?;
                            }
                            bar.focus
                        };
                        check(
                            landing < model.entries.len() && model.entries[landing].enabled,
                            || format!("activated item {landing}, which cannot be selected"),
                        )?;
                        model.sel = Some(model.entries[landing].id);
                        model.touched = true;
                        model.mode = Mode::Selected;
                    }
                }
            } else if roll < 90 {
                // A key, as the root view sends it: only while the bar is selected.
                let key = [
                    Key::Left,
                    Key::Right,
                    Key::Right,
                    Key::Down,
                    Key::Enter,
                    Key::Left,
                ][rng.below(6)];
                if model.mode != Mode::Selected {
                    log.push(format!("{key:?} (not selected, not sent)"));
                    continue;
                }
                log.push(format!("{key:?}"));
                let result = bar.on_event(Event::Key(key));
                let from = model.position(model.sel.unwrap());
                let kind = model.entries[from].kind;
                match key {
                    Key::Left | Key::Right => {
                        // The closest enabled item in that direction. Going all the way
                        // around gives the same item back.
                        let target = (1..=model.entries.len())
                            .map(|k| model.step(from, k, key == Key::Right))
                            .find(|&j| model.entries[j].enabled);
                        check(result.is_consumed() == target.is_some(), || {
                            format!("consumed is {}", result.is_consumed())
                        })?;
                        if let Some(target) = target {
                            model.sel = Some(model.entries[target].id);
                        }
                    }
                    _ => match (kind, key) {
                        (Kind::Subtree, _) => {
                            check(result.is_consumed(), || "a subtree did not open".into())?;
                            model.mode = Mode::Submenu;
                        }
                        (_, Key::Enter) => {
                            check(result.is_consumed(), || "a leaf was not activated".into())?;
                            model.mode = Mode::Inactive;
                        }
                        _ => check(!result.is_consumed(), || {
                            "Down on a leaf was consumed".into()
                        })?,
                    },
                }
            } else if roll < 94 {
                log.push("Esc".into());
                let result = bar.on_event(Event::Key(Key::Esc));
                check(result.is_consumed(), || "Esc was ignored".into())?;
                model.mode = Mode::Inactive;
            } else if roll < 98 {
                // A popup was dismissed or acted on.
                log.push("hide".into());
                bar.hide();
                model.mode = Mode::Inactive;
            } else {
                log.push("clear".into());
                bar.clear();
                model.entries.clear();
                model.sel = None;
                model.touched = false;
                model.removed_untouched = false;
                model.mode = Mode::Inactive;
            }
            check_invariants(&bar, &model)?;
        }
        Ok(())
    }

    fn fuzz(runs: u64, strict: bool) {
        for seed in 0..runs {
            let mut log = Vec::new();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_menubar_fuzz(seed, 40, strict, &mut log)
            }));
            match outcome {
                Ok(Ok(())) => (),
                Ok(Err(error)) => panic!("seed {seed}: {error}\noperations: {log:#?}"),
                Err(_) => panic!("seed {seed}: panic\noperations: {log:#?}"),
            }
        }
    }

    #[test]
    fn random_operations_keep_the_selection_consistent() {
        fuzz(4000, false);
    }

    #[test]
    fn random_operations_on_a_never_activated_menubar_land_on_the_leftmost_item() {
        fuzz(4000, true);
    }

    #[test]
    fn first_activation_after_a_removal_is_the_leftmost_item() {
        let mut bar = Menubar::new();
        bar.item(leaf("off").disabled())
            .add_leaf("e", |_| {})
            .add_leaf("f", |_| {});
        // The default position moves to the first enabled item...
        bar.remove(2);
        // ...and stays there when an item is inserted before it.
        bar.insert_leaf(0, "new", |_| {});
        bar.take_focus(direction::Direction::none()).unwrap();
        assert_eq!(bar.root.children[bar.focus].label(), "new");
    }
}
