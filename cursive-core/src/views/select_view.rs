use crate::{
    align::{Align, HAlign, VAlign},
    direction,
    event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent},
    menu,
    rect::Rect,
    style::{PaletteStyle, Style, StyleType},
    utils::markup::StyledString,
    view::{CannotFocus, Position, View},
    views::{LayerPosition, MenuPopup},
    Cursive, Printer, Vec2, With,
};
use std::borrow::Borrow;
use std::cmp::{min, Ordering};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use unicode_width::UnicodeWidthStr;

type SelectCallback<T> = dyn Fn(&mut Cursive, &T) + Send + Sync;

/// View to select an item among a list.
///
/// It contains a list of values of type T, with associated labels.
///
/// # Examples
///
/// ```rust
/// # use cursive_core::Cursive;
/// # use cursive_core::views::{SelectView, Dialog, TextView};
/// # use cursive_core::align::HAlign;
/// let mut time_select = SelectView::new().h_align(HAlign::Center);
/// time_select.add_item("Short", 1);
/// time_select.add_item("Medium", 5);
/// time_select.add_item("Long", 10);
///
/// time_select.set_on_submit(|s, time| {
///     s.pop_layer();
///     let text = format!("You will wait for {} minutes...", time);
///     s.add_layer(Dialog::around(TextView::new(text)).button("Quit", |s| s.quit()));
/// });
///
/// let mut siv = Cursive::new();
/// siv.add_layer(Dialog::around(time_select).title("How long is your wait?"));
/// ```
pub struct SelectView<T = String> {
    // The core of the view: we store a list of items
    // `Item` is more or less a `(String, Arc<T>)`.
    items: Vec<Item<T>>,

    // When disabled, we cannot change selection.
    enabled: bool,

    // Callbacks may need to manipulate focus, so give it some mutability.
    focus: Arc<AtomicUsize>,

    // If true, highlight the selection even when inactive (not focused).
    // If false, selection will be drawn like regular text if inactive.
    inactive_highlight: bool,

    // This is a custom callback to include a &T.
    // It will be called whenever "Enter" is pressed or when an item is clicked.
    on_submit: Option<Arc<SelectCallback<T>>>,

    // This callback is called when the selection is changed.
    // TODO: add the previous selection? Indices?
    on_select: Option<Arc<SelectCallback<T>>>,

    // If `true`, when a character is pressed, jump to the next item starting
    // with this character.
    autojump: bool,

    align: Align,

    // `true` if we show a one-line view, with popup on selection.
    popup: bool,
    // Decorators to draw around the popup button.
    decorators: [String; 2],

    // We need the last offset to place the popup window
    // We "cache" it during the draw, so we need interior mutability.
    last_offset: Mutex<Vec2>,
    last_size: Vec2,

    // Cache of required_size. Set to None when it needs to be recomputed.
    last_required_size: Option<Vec2>,
}

impl<T: 'static + Send + Sync> Default for SelectView<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static + Send + Sync> SelectView<T> {
    impl_enabled!(self.enabled);

    /// Creates a new empty SelectView.
    pub fn new() -> Self {
        SelectView {
            items: Vec::new(),
            enabled: true,
            focus: Arc::new(AtomicUsize::new(0)),
            inactive_highlight: true,
            on_select: None,
            on_submit: None,
            align: Align::top_left(),
            popup: false,
            decorators: ["<".to_string(), ">".to_string()],
            autojump: false,
            last_offset: Mutex::new(Vec2::zero()),
            last_size: Vec2::zero(),
            last_required_size: None,
        }
    }

    /// Sets the "auto-jump" property for this view.
    ///
    /// If enabled, when a key is pressed, the selection will jump to the next
    /// item beginning with the pressed letter.
    pub fn set_autojump(&mut self, autojump: bool) {
        self.autojump = autojump;
    }

    /// Sets the "auto-jump" property for this view.
    ///
    /// If enabled, when a key is pressed, the selection will jump to the next
    /// item beginning with the pressed letter.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn autojump(self) -> Self {
        self.with(|s| s.set_autojump(true))
    }

    /// Sets the "inactive highlight" property for this view.
    ///
    /// * If true (the default), the selected row will be highlighted when the
    ///   view is not focused.
    /// * If false, the selected row will be printed like the others if inactive.
    pub fn set_inactive_highlight(&mut self, inactive_highlight: bool) {
        self.inactive_highlight = inactive_highlight;
    }

    /// Sets the "inactive highlight" property for this view.
    ///
    /// * If true (the default), the selected row will be highlighted when the
    ///   view is not focused.
    /// * If false, the selected row will be printed like the others if inactive.
    ///
    /// Chainable variant.
    pub fn with_inactive_highlight(self, inactive_highlight: bool) -> Self {
        self.with(|s| s.set_inactive_highlight(inactive_highlight))
    }

    /// Returns the current status of the "inactive highlight" property.
    pub fn get_inactive_highlight(&self) -> bool {
        self.inactive_highlight
    }

    /// Turns `self` into a popup select view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn popup(self) -> Self {
        self.with(|s| s.set_popup(true))
    }

    /// Turns `self` into a popup select view.
    pub fn set_popup(&mut self, popup: bool) {
        self.popup = popup;
        self.last_required_size = None;
    }

    /// Use custom decorators around the popup button instead of "<" and ">".
    ///
    /// Chainable variant.
    #[must_use]
    pub fn decorators<S: Into<String>>(self, start: S, end: S) -> Self {
        self.with(|s| s.set_decorators(start, end))
    }

    /// Use custom decorators around the popup button instead of "<" and ">".
    pub fn set_decorators<S: Into<String>>(&mut self, start: S, end: S) {
        self.decorators = [start.into(), end.into()];
    }

    /// Sets a callback to be used when an item is selected.
    #[crate::callback_helpers]
    pub fn set_on_select<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive, &T) + 'static + Send + Sync,
    {
        self.on_select = Some(Arc::new(cb));
    }

    /// Sets a callback to be used when an item is selected.
    ///
    /// Chainable variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::traits::Nameable;
    /// use cursive_core::views::{SelectView, TextView};
    ///
    /// let text_view = TextView::new("").with_name("text");
    ///
    /// let select_view = SelectView::new()
    ///     .item("One", 1)
    ///     .item("Two", 2)
    ///     .on_select(|s, item| {
    ///         let content = match *item {
    ///             1 => "Content number one",
    ///             2 => "Content number two! Much better!",
    ///             _ => unreachable!("no such item"),
    ///         };
    ///
    ///         // Update the textview with the currently selected item.
    ///         s.call_on_name("text", |v: &mut TextView| {
    ///             v.set_content(content);
    ///         })
    ///         .unwrap();
    ///     });
    /// ```
    #[must_use]
    pub fn on_select<F>(self, cb: F) -> Self
    where
        F: Fn(&mut Cursive, &T) + 'static + Send + Sync,
    {
        self.with(|s| s.set_on_select(cb))
    }

    /// Sets a callback to be used when `<Enter>` is pressed.
    ///
    /// Also happens if the user clicks an item.
    ///
    /// The item currently selected will be given to the callback.
    ///
    /// Here, `V` can be `T` itself, or a type that can be borrowed from `T`.
    pub fn set_on_submit<F, V: ?Sized>(&mut self, cb: F)
    where
        F: 'static + Fn(&mut Cursive, &V) + Send + Sync,
        T: Borrow<V>,
    {
        self.on_submit = Some(Arc::new(move |s, t| {
            cb(s, t.borrow());
        }));
    }

    /// Sets a callback to be used when `<Enter>` is pressed.
    ///
    /// Also happens if the user clicks an item.
    ///
    /// The item currently selected will be given to the callback.
    ///
    /// Chainable variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::{Dialog, SelectView};
    ///
    /// let select_view = SelectView::new()
    ///     .item("One", 1)
    ///     .item("Two", 2)
    ///     .on_submit(|s, item| {
    ///         let content = match *item {
    ///             1 => "Content number one",
    ///             2 => "Content number two! Much better!",
    ///             _ => unreachable!("no such item"),
    ///         };
    ///
    ///         // Show a popup whenever the user presses <Enter>.
    ///         s.add_layer(Dialog::info(content));
    ///     });
    /// ```
    #[must_use]
    pub fn on_submit<F, V: ?Sized>(self, cb: F) -> Self
    where
        F: Fn(&mut Cursive, &V) + 'static + Send + Sync,
        T: Borrow<V>,
    {
        self.with(|s| s.set_on_submit(cb))
    }

    /// Sets the alignment for this view.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::align;
    /// use cursive_core::views::SelectView;
    ///
    /// let select_view = SelectView::new()
    ///     .item("One", 1)
    ///     .align(align::Align::top_center());
    /// ```
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;

        self
    }

    /// Sets the vertical alignment for this view.
    /// (If the view is given too much space vertically.)
    #[must_use]
    pub fn v_align(mut self, v: VAlign) -> Self {
        self.align.v = v;

        self
    }

    /// Sets the horizontal alignment for this view.
    #[must_use]
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
    }

    /// Returns the value of the currently selected item.
    ///
    /// Returns `None` if the list is empty.
    pub fn selection(&self) -> Option<Arc<T>> {
        let focus = self.focus();
        if self.len() <= focus {
            None
        } else {
            Some(Arc::clone(&self.items[focus].value))
        }
    }

    /// Removes all items from this view.
    pub fn clear(&mut self) {
        self.items.clear();
        self.focus.store(0, std::sync::atomic::Ordering::Relaxed);
        self.last_required_size = None;
    }

    /// Adds a item to the list, with given label and value.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::SelectView;
    ///
    /// let mut select_view = SelectView::new();
    ///
    /// select_view.add_item("Item 1", 1);
    /// select_view.add_item("Item 2", 2);
    /// ```
    pub fn add_item<S: Into<StyledString>>(&mut self, label: S, value: T) {
        self.items.push(Item::new(label.into(), value));
        self.last_required_size = None;
    }

    /// Gets an item at given idx or None.
    ///
    /// ```
    /// use cursive_core::views::{SelectView, TextView};
    /// use cursive_core::Cursive;
    /// let select = SelectView::new().item("Short", 1);
    /// assert_eq!(select.get_item(0), Some(("Short", &1)));
    /// ```
    pub fn get_item(&self, i: usize) -> Option<(&str, &T)> {
        self.iter().nth(i)
    }

    /// Gets a mut item at given idx or None.
    pub fn get_item_mut(&mut self, i: usize) -> Option<(&mut StyledString, &mut T)> {
        if i >= self.items.len() {
            None
        } else {
            self.last_required_size = None;
            let item = &mut self.items[i];
            if let Some(t) = Arc::get_mut(&mut item.value) {
                let label = &mut item.label;
                Some((label, t))
            } else {
                None
            }
        }
    }

    /// Iterate mutably on the items in this view.
    ///
    /// Returns an iterator with each item and their labels.
    ///
    /// In some cases some items will need to be cloned (for example if a
    /// `Arc<T>` is still alive after calling `SelectView::selection()`).
    ///
    /// If `T` does not implement `Clone`, check `SelectView::try_iter_mut()`.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut StyledString, &mut T)>
    where
        T: Clone,
    {
        self.last_required_size = None;
        self.items
            .iter_mut()
            .map(|item| (&mut item.label, Arc::make_mut(&mut item.value)))
    }

    /// Try to iterate mutably on the items in this view.
    ///
    /// Returns an iterator with each item and their labels.
    ///
    /// Some items may not be returned mutably, for example if a `Arc<T>` is
    /// still alive after calling `SelectView::selection()`.
    pub fn try_iter_mut(&mut self) -> impl Iterator<Item = (&mut StyledString, Option<&mut T>)> {
        self.last_required_size = None;
        self.items
            .iter_mut()
            .map(|item| (&mut item.label, Arc::get_mut(&mut item.value)))
    }

    /// Iterate on the items in this view.
    ///
    /// Returns an iterator with each item and their labels.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &T)> {
        self.items
            .iter()
            .map(|item| (item.label.source(), &*item.value))
    }

    /// Removes an item from the list.
    ///
    /// Returns a callback in response to the selection change.
    ///
    /// You should run this callback with a `&mut Cursive`.
    pub fn remove_item(&mut self, id: usize) -> Callback {
        self.items.remove(id);
        self.last_required_size = None;
        let focus = self.focus();
        (focus >= id && focus > 0)
            .then(|| {
                self.set_focus(focus - 1);
                self.make_select_cb()
            })
            .flatten()
            .unwrap_or_else(Callback::dummy)
    }

    /// Inserts an item at position `index`, shifting all elements after it to
    /// the right.
    pub fn insert_item<S>(&mut self, index: usize, label: S, value: T)
    where
        S: Into<StyledString>,
    {
        self.items.insert(index, Item::new(label.into(), value));
        let focus = self.focus();
        // Do not increase focus if we were empty with focus=0.
        if focus >= index && !self.items.is_empty() {
            self.set_focus(focus + 1);
        }
        self.last_required_size = None;
    }

    /// Chainable variant of add_item
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::SelectView;
    ///
    /// let select_view = SelectView::new()
    ///     .item("Item 1", 1)
    ///     .item("Item 2", 2)
    ///     .item("Surprise item", 42);
    /// ```
    #[must_use]
    pub fn item<S: Into<StyledString>>(self, label: S, value: T) -> Self {
        self.with(|s| s.add_item(label, value))
    }

    /// Adds all items from from an iterator.
    pub fn add_all<S, I>(&mut self, iter: I)
    where
        S: Into<StyledString>,
        I: IntoIterator<Item = (S, T)>,
    {
        for (s, t) in iter {
            self.add_item(s, t);
        }
    }

    /// Adds all items from from an iterator.
    ///
    /// Chainable variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::SelectView;
    ///
    /// // Create a SelectView with 100 items
    /// let select_view =
    ///     SelectView::new().with_all((1u8..100).into_iter().map(|i| (format!("Item {}", i), i)));
    /// ```
    #[must_use]
    pub fn with_all<S, I>(self, iter: I) -> Self
    where
        S: Into<StyledString>,
        I: IntoIterator<Item = (S, T)>,
    {
        self.with(|s| s.add_all(iter))
    }

    fn draw_item(&self, printer: &Printer, i: usize) {
        let l = self.items[i].label.width();
        let x = self.align.h.get_offset(l, printer.size.x);
        printer.print_hline((0, 0), x, " ");
        printer.print_styled((x, 0), &self.items[i].label);
        if l < printer.size.x {
            assert!((l + x) <= printer.size.x);
            printer.print_hline((x + l, 0), printer.size.x - (l + x), " ");
        }
    }

    /// Returns the id of the item currently selected.
    ///
    /// Returns `None` if the list is empty.
    pub fn selected_id(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.focus())
        }
    }

    /// Returns the number of items in this list.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::SelectView;
    ///
    /// let select_view = SelectView::new()
    ///     .item("Item 1", 1)
    ///     .item("Item 2", 2)
    ///     .item("Item 3", 3);
    ///
    /// assert_eq!(select_view.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if this list has no item.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::SelectView;
    ///
    /// let mut select_view = SelectView::new();
    /// assert!(select_view.is_empty());
    ///
    /// select_view.add_item("Item 1", 1);
    /// select_view.add_item("Item 2", 2);
    /// assert!(!select_view.is_empty());
    ///
    /// select_view.clear();
    /// assert!(select_view.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn focus(&self) -> usize {
        self.focus.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn set_focus(&mut self, focus: usize) {
        self.focus
            .store(focus, std::sync::atomic::Ordering::Relaxed);
    }

    /// Sort the current items lexicographically by their label.
    ///
    /// Note that this does not change the current focus index, which means that the current
    /// selection will likely be changed by the sorting.
    ///
    /// This sort is stable: items with identical label will not be reordered.
    pub fn sort_by_label(&mut self) {
        self.items
            .sort_by(|a, b| a.label.source().cmp(b.label.source()));
    }

    /// Sort the current items with the given comparator function.
    ///
    /// Note that this does not change the current focus index, which means that the current
    /// selection will likely be changed by the sorting.
    ///
    /// The given comparator function must define a total order for the items.
    ///
    /// If the comparator function does not define a total order, then the order after the sort is
    /// unspecified.
    ///
    /// This sort is stable: equal items will not be reordered.
    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.items.sort_by(|a, b| compare(&a.value, &b.value));
    }

    /// Sort the current items with the given key extraction function.
    ///
    /// Note that this does not change the current focus index, which means that the current
    /// selection will likely be changed by the sorting.
    ///
    /// This sort is stable: items with equal keys will not be reordered.
    pub fn sort_by_key<K, F>(&mut self, mut key_of: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        self.items.sort_by_key(|item| key_of(&item.value));
    }

    /// Moves the selection to the given position.
    ///
    /// Returns a callback in response to the selection change.
    ///
    /// You should run this callback with a `&mut Cursive`.
    pub fn set_selection(&mut self, i: usize) -> Callback {
        // TODO: Check if `i >= self.len()` ?
        // assert!(i < self.len(), "SelectView: trying to select out-of-bound");
        // Or just cap the ID?
        let i = if self.is_empty() {
            0
        } else {
            min(i, self.len() - 1)
        };
        self.set_focus(i);

        self.make_select_cb().unwrap_or_else(Callback::dummy)
    }

    /// Sets the selection to the given position.
    ///
    /// Chainable variant.
    ///
    /// Does not apply `on_select` callbacks.
    #[must_use]
    pub fn selected(self, i: usize) -> Self {
        self.with(|s| {
            s.set_selection(i);
        })
    }

    /// Moves the selection up by the given number of rows.
    ///
    /// Returns a callback in response to the selection change.
    ///
    /// You should run this callback with a `&mut Cursive`:
    ///
    /// ```rust
    /// # use cursive_core::Cursive;
    /// # use cursive_core::views::SelectView;
    /// fn select_up(siv: &mut Cursive, view: &mut SelectView<()>) {
    ///     let cb = view.select_up(1);
    ///     cb(siv);
    /// }
    /// ```
    pub fn select_up(&mut self, n: usize) -> Callback {
        self.focus_up(n);
        self.make_select_cb().unwrap_or_else(Callback::dummy)
    }

    /// Moves the selection down by the given number of rows.
    ///
    /// Returns a callback in response to the selection change.
    ///
    /// You should run this callback with a `&mut Cursive`.
    pub fn select_down(&mut self, n: usize) -> Callback {
        self.focus_down(n);
        self.make_select_cb().unwrap_or_else(Callback::dummy)
    }

    fn focus_up(&mut self, n: usize) {
        let focus = self.focus().saturating_sub(n);
        self.set_focus(focus);
    }

    fn focus_down(&mut self, n: usize) {
        let focus = min(self.focus() + n, self.items.len().saturating_sub(1));
        self.set_focus(focus);
    }

    fn submit(&mut self) -> EventResult {
        let cb = self.on_submit.clone().unwrap();
        // We return a Callback Arc<|s| cb(s, &*v)>
        EventResult::Consumed(
            self.selection()
                .map(|v| Callback::from_fn(move |s| cb(s, &v))),
        )
    }

    fn on_char_event(&mut self, c: char) -> EventResult {
        let i = {
            // * Starting from the current focus, find the first item that
            //   match the char.
            // * Cycle back to the beginning of the list when we reach the end.
            // * This is achieved by chaining twice the iterator.
            let iter = self.iter().chain(self.iter());

            // We'll do a lowercase check.
            let lower_c: Vec<char> = c.to_lowercase().collect();
            let lower_c: &[char] = &lower_c;

            if let Some((i, _)) = iter
                .enumerate()
                .skip(self.focus() + 1)
                .find(|&(_, (label, _))| label.to_lowercase().starts_with(lower_c))
            {
                i % self.len()
            } else {
                return EventResult::Ignored;
            }
        };

        self.set_focus(i);
        // Apply modulo in case we have a hit from the chained iterator
        let cb = self.set_selection(i);
        EventResult::Consumed(Some(cb))
    }

    fn on_event_regular(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Up) if self.focus() > 0 => self.focus_up(1),
            Event::Key(Key::Down) if self.focus() + 1 < self.items.len() => self.focus_down(1),
            Event::Key(Key::PageUp) => self.focus_up(10),
            Event::Key(Key::PageDown) => self.focus_down(10),
            Event::Key(Key::Home) => self.set_focus(0),
            Event::Key(Key::End) => self.set_focus(self.items.len().saturating_sub(1)),
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } if position
                .checked_sub(offset)
                .map(|position| position < self.last_size && position.y < self.len())
                .unwrap_or(false) =>
            {
                self.set_focus(position.y - offset.y)
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if self.on_submit.is_some()
                && position
                    .checked_sub(offset)
                    .map(|position| position < self.last_size && position.y == self.focus())
                    .unwrap_or(false) =>
            {
                return self.submit();
            }
            Event::Key(Key::Enter) if self.on_submit.is_some() => {
                return self.submit();
            }
            Event::Char(c) if self.autojump => return self.on_char_event(c),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(self.make_select_cb())
    }

    /// Returns a callback from selection change.
    fn make_select_cb(&self) -> Option<Callback> {
        self.on_select.clone().and_then(|cb| {
            self.selection()
                .map(|v| Callback::from_fn(move |s| cb(s, &v)))
        })
    }

    fn open_popup(&mut self) -> EventResult {
        // Build a shallow menu tree to mimic the items array.
        // TODO: cache it?
        let mut tree = menu::Tree::new();
        for (i, item) in self.items.iter().enumerate() {
            let focus = Arc::clone(&self.focus);
            let on_submit = self.on_submit.as_ref().cloned();
            let value = Arc::clone(&item.value);
            tree.add_leaf(item.label.source(), move |s| {
                // TODO: What if an item was removed in the meantime?
                focus.store(i, std::sync::atomic::Ordering::Relaxed);
                if let Some(ref on_submit) = on_submit {
                    on_submit(s, &value);
                }
            });
        }
        // Let's keep the tree around,
        // the callback will want to use it.
        let tree = Arc::new(tree);

        let focus = self.focus();
        // This is the offset for the label text.
        // We'll want to show the popup so that the text matches.
        // It'll be soo cool.
        let item_length = self.items[focus].label.width();
        let text_offset = self
            .last_size
            .x
            .saturating_sub(self.decorators_width())
            .saturating_sub(item_length)
            / 2
            + self.decorators[0].width();
        // The total offset for the window is:
        // * the last absolute offset at which we drew this view
        // * shifted to the right of the text offset
        // * shifted to the top of the focus (so the line matches)
        // * shifted top-left of the border+padding of the popup
        let offset = *self.last_offset.lock().unwrap();
        let offset = offset + (text_offset, 0);
        let offset = offset.saturating_sub((0, focus));
        let offset = offset.saturating_sub((2, 1));

        // And now, we can return the callback that will create the popup.
        EventResult::with_cb(move |s| {
            // The callback will want to work with a fresh Arc
            let tree = Arc::clone(&tree);
            // We'll relativise the absolute position,
            // So that we are locked to the parent view.
            // A nice effect is that window resizes will keep both
            // layers together.
            let current_offset = s
                .screen()
                .layer_offset(LayerPosition::FromFront(0))
                .unwrap_or_else(Vec2::zero);
            let offset = offset.signed() - current_offset;
            // And finally, put the view in view!
            s.screen_mut()
                .add_layer_at(Position::parent(offset), MenuPopup::new(tree).focus(focus));
        })
    }

    // A popup view opens the popup on Enter, and also applies autojump if
    // enabled.
    fn on_event_popup(&mut self, event: Event) -> EventResult {
        match event {
            // TODO: add Left/Right support for quick-switch?
            Event::Key(Key::Enter) => self.open_popup(),
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset, self.last_size) => self.open_popup(),
            Event::Char(c) if self.autojump => self.on_char_event(c),
            _ => EventResult::Ignored,
        }
    }

    fn decorators_width(&self) -> usize {
        self.decorators.iter().map(|d| d.width()).sum()
    }
}

impl SelectView<String> {
    /// Convenient method to use the label as value.
    pub fn add_item_str<S: Into<String>>(&mut self, label: S) {
        let label = label.into();
        self.add_item(label.clone(), label);
    }

    /// Convenient method to use the label unstyled text as value.
    pub fn add_item_styled<S: Into<StyledString>>(&mut self, label: S) {
        let label = label.into();

        // Accumulate the content of each span.
        let mut content = String::new();
        for span in label.spans() {
            content.push_str(span.content);
        }

        self.add_item(label, content);
    }

    /// Chainable variant of `add_item_str`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::SelectView;
    ///
    /// let select_view = SelectView::new()
    ///     .item_str("Paris")
    ///     .item_str("New York")
    ///     .item_str("Tokyo");
    /// ```
    #[must_use]
    pub fn item_str<S: Into<String>>(self, label: S) -> Self {
        self.with(|s| s.add_item_str(label))
    }

    /// Chainable variant of `add_item_styled`.
    #[must_use]
    pub fn item_styled<S: Into<StyledString>>(self, label: S) -> Self {
        self.with(|s| s.add_item_styled(label))
    }

    /// Convenient method to use the label as value.
    pub fn insert_item_str<S>(&mut self, index: usize, label: S)
    where
        S: Into<String>,
    {
        let label = label.into();
        self.insert_item(index, label.clone(), label);
    }

    /// Adds all strings from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cursive_core::views::SelectView;
    /// let mut select_view = SelectView::new();
    /// select_view.add_all_str(vec!["a", "b", "c"]);
    /// ```
    pub fn add_all_str<S, I>(&mut self, iter: I)
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        for s in iter {
            self.add_item_str(s);
        }
    }

    /// Adds all strings from an iterator.
    ///
    /// Chainable variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::SelectView;
    ///
    /// let text = "..."; // Maybe read some config file
    ///
    /// let select_view = SelectView::new().with_all_str(text.lines());
    /// ```
    #[must_use]
    pub fn with_all_str<S, I>(self, iter: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        self.with(|s| s.add_all_str(iter))
    }
}

impl<T: 'static> SelectView<T>
where
    T: Ord,
{
    /// Sort the current items by their natural ordering.
    ///
    /// Note that this does not change the current focus index, which means that the current
    /// selection will likely be changed by the sorting.
    ///
    /// This sort is stable: items that are equal will not be reordered.
    pub fn sort(&mut self) {
        self.items.sort_by(|a, b| a.value.cmp(&b.value));
    }
}

impl<T: 'static + Send + Sync> View for SelectView<T> {
    fn draw(&self, printer: &Printer) {
        *self.last_offset.lock().unwrap() = printer.offset;

        let focus = self.focus();

        if self.popup {
            // Popup-select only draw the active element.
            // We'll draw the full list in a popup if needed.
            let style = if !(self.enabled && printer.enabled) {
                PaletteStyle::Secondary
            } else if printer.focused {
                PaletteStyle::Highlight
            } else {
                PaletteStyle::Primary
            };

            let available = match printer.size.x.checked_sub(self.decorators_width()) {
                Some(available) => available,
                None => return,
            };

            printer.with_style(style, |printer| {
                let decorator0_width = self.decorators[0].width();
                // Prepare the label background
                printer.print_hline((decorator0_width, 0), available, " ");
                // Draw the decorators
                printer.print((0, 0), &self.decorators[0]);
                printer.print((decorator0_width + available, 0), &self.decorators[1]);

                if let Some(label) = self.items.get(focus).map(|item| &item.label) {
                    // And center the text?
                    let offset =
                        decorator0_width + HAlign::Center.get_offset(label.width(), available);

                    printer.print_styled((offset, 0), label);
                }
            });
        } else {
            // Non-popup mode: we always print the entire list.
            let h = self.items.len();
            let offset = self.align.v.get_offset(h, printer.size.y);
            let printer = &printer.offset((0, offset));

            let enabled = self.enabled && printer.enabled;
            let active = printer.focused;

            let regular_style: StyleType = if enabled {
                Style::inherit_parent().into()
            } else {
                PaletteStyle::Secondary.into()
            };

            let highlight_style = if active {
                PaletteStyle::Highlight.into()
            } else if self.inactive_highlight {
                PaletteStyle::HighlightInactive.into()
            } else {
                regular_style
            };

            for i in 0..self.len() {
                let style = if i == focus {
                    highlight_style
                } else {
                    regular_style
                };

                printer.offset((0, i)).with_style(style, |printer| {
                    self.draw_item(printer, i);
                });
            }
        }
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        if let Some(s) = self.last_required_size {
            return s;
        }
        // Items here are not compressible.
        // So no matter what the horizontal requirements are,
        // we'll still return our longest item.
        let w = self
            .items
            .iter()
            .map(|item| item.label.width())
            .max()
            .unwrap_or(1);
        let size = if self.popup {
            Vec2::new(w + self.decorators_width(), 1)
        } else {
            let h = self.items.len();

            Vec2::new(w, h)
        };
        self.last_required_size = Some(size);
        size
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }

        if self.popup {
            self.on_event_popup(event)
        } else {
            self.on_event_regular(event)
        }
    }

    fn take_focus(&mut self, source: direction::Direction) -> Result<EventResult, CannotFocus> {
        (self.enabled && !self.items.is_empty())
            .then(|| {
                if !self.popup {
                    match source {
                        direction::Direction::Abs(direction::Absolute::Up) => {
                            self.set_focus(0);
                        }
                        direction::Direction::Abs(direction::Absolute::Down) => {
                            self.set_focus(self.items.len().saturating_sub(1));
                        }
                        _ => (),
                    }
                }
                EventResult::Consumed(None)
            })
            .ok_or(CannotFocus)
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;
    }

    fn important_area(&self, size: Vec2) -> Rect {
        self.selected_id()
            .map(|i| Rect::from_size((0, i), (size.x, 1)))
            .unwrap_or_else(|| Rect::from_size(Vec2::zero(), size))
    }
}

// We wrap each value in a `Arc` and add a label
struct Item<T> {
    label: StyledString,
    value: Arc<T>,
}

impl<T> Item<T> {
    fn new(label: StyledString, value: T) -> Self {
        let value = Arc::new(value);
        Item { label, value }
    }
}

#[crate::blueprint(SelectView::<String>::new())]
struct Blueprint {
    autojump: Option<bool>,
    popup: Option<bool>,

    on_select: Option<_>,

    #[blueprint(foreach = add_item_str)]
    items: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_view_sorting() {
        // We add items in no particular order, from going by their label.
        let mut view = SelectView::new();
        view.add_item_str("Y");
        view.add_item_str("Z");
        view.add_item_str("X");

        // Then sorting the list...
        view.sort_by_label();

        // ... should observe the items in sorted order.
        // And focus is NOT changed by the sorting, so the first item is "X".
        assert_eq!(view.selection(), Some(Arc::new(String::from("X"))));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(String::from("Y"))));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(String::from("Z"))));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(String::from("Z"))));
    }

    #[test]
    fn select_view_sorting_with_comparator() {
        // We add items in no particular order, from going by their value.
        let mut view = SelectView::new();
        view.add_item("Y", 2);
        view.add_item("Z", 1);
        view.add_item("X", 3);

        // Then sorting the list...
        view.sort_by(|a, b| a.cmp(b));

        // ... should observe the items in sorted order.
        // And focus is NOT changed by the sorting, so the first item is "X".
        assert_eq!(view.selection(), Some(Arc::new(1)));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(2)));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(3)));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(3)));
    }

    #[test]
    fn select_view_sorting_by_key() {
        // We add items in no particular order, from going by their key value.
        #[derive(Eq, PartialEq, Debug)]
        struct MyStruct {
            key: i32,
        }

        let mut view = SelectView::new();
        view.add_item("Y", MyStruct { key: 2 });
        view.add_item("Z", MyStruct { key: 1 });
        view.add_item("X", MyStruct { key: 3 });

        // Then sorting the list...
        view.sort_by_key(|s| s.key);

        // ... should observe the items in sorted order.
        // And focus is NOT changed by the sorting, so the first item is "X".
        assert_eq!(view.selection(), Some(Arc::new(MyStruct { key: 1 })));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(MyStruct { key: 2 })));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(MyStruct { key: 3 })));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(MyStruct { key: 3 })));
    }

    #[test]
    fn select_view_sorting_orderable_items() {
        // We add items in no particular order, from going by their value.
        let mut view = SelectView::new();
        view.add_item("Y", 2);
        view.add_item("Z", 1);
        view.add_item("X", 3);

        // Then sorting the list...
        view.sort();

        // ... should observe the items in sorted order.
        // And focus is NOT changed by the sorting, so the first item is "X".
        assert_eq!(view.selection(), Some(Arc::new(1)));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(2)));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(3)));
        view.on_event(Event::Key(Key::Down));
        assert_eq!(view.selection(), Some(Arc::new(3)));
    }
}
