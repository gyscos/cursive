use ahash::{HashSet, HashSetExt};

use crate::{
    direction::Direction,
    event::{Event, EventResult, Key, MouseButton, MouseEvent},
    style::PaletteStyle,
    utils::markup::StyledString,
    view::{CannotFocus, View},
    Cursive, Printer, Vec2, With,
};
use parking_lot::Mutex;
use std::hash::Hash;
use std::sync::Arc;

type GroupCallback<T> = dyn Fn(&mut Cursive, &HashSet<Arc<T>>) + Send + Sync;
type Callback = dyn Fn(&mut Cursive, bool) + Send + Sync;

struct Item<T> {
    value: Arc<T>,
    checked: bool,
}

// We have to manually implement Clone.
// Using derive(Clone) would add am unwanted `T: Clone` where-clause.
impl<T> Clone for Item<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            checked: self.checked,
        }
    }
}

struct SharedState<T> {
    items: Vec<Item<T>>,

    on_change: Option<Arc<GroupCallback<T>>>,
}

impl<T> SharedState<T> {
    fn add(&mut self, value: T, checked: bool) -> usize {
        let value = Arc::new(value);
        let i = self.items.len();
        self.items.push(Item { value, checked });
        i
    }

    fn set_checked(&mut self, i: usize, checked: bool) {
        self.items[i].checked = checked;
    }

    fn selections(&self) -> Vec<Arc<T>> {
        self.items
            .iter()
            .filter(|item| item.checked)
            .cloned()
            .map(|item| item.value)
            .collect()
    }
}

/// Group to coordinate multiple checkboxes.
///
/// A `MultiChoiceGroup` can be used to create and manage multiple [`Checkbox`]es.
///
/// A `MultiChoiceGroup` can be cloned; it will keep shared state (pointing to the same group).
pub struct MultiChoiceGroup<T> {
    // Given to every child button
    state: Arc<Mutex<SharedState<T>>>,
}

// We have to manually implement Clone.
// Using derive(Clone) would add am unwanted `T: Clone` where-clause.
impl<T> Clone for MultiChoiceGroup<T> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

impl<T: 'static + Hash + Eq> Default for MultiChoiceGroup<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static + Hash + Eq> MultiChoiceGroup<T> {
    /// Creates an empty group for check boxes.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SharedState {
                items: Vec::new(),
                on_change: None,
            })),
        }
    }

    /// Adds a new checkbox to the group.
    ///
    /// The checkbox will display `label` next to it, and will ~embed~ `value`.
    pub fn checkbox<S: Into<StyledString>>(&mut self, value: T, label: S) -> Checkbox
    where
        T: Send + Sync,
    {
        let i = self.state.lock().add(value, false);
        Checkbox::labelled(label).on_change({
            let groupstate = Arc::clone(&self.state);
            move |_, checked| {
                groupstate.lock().set_checked(i, checked);
            }
        })
    }

    /// Returns the reference to a vector associated with the selected checkboxes.
    pub fn selections(&self) -> Vec<Arc<T>> {
        self.state.lock().selections()
    }

    /// Sets a callback to be user when choices change.
    pub fn set_on_change<F>(&mut self, on_change: F)
    where
        F: Send + Sync + 'static + Fn(&mut Cursive, &HashSet<Arc<T>>),
    {
        self.state.lock().on_change = Some(Arc::new(on_change));
    }

    /// Set a callback to use used when choices change.
    ///
    /// Chainable variant.
    pub fn on_change<F>(self, on_change: F) -> Self
    where
        F: Send + Sync + 'static + Fn(&mut Cursive, &HashSet<Arc<T>>),
    {
        crate::With::with(self, |s| s.set_on_change(on_change))
    }
}

/// Checkable box.
///
/// # Examples
///
/// ```rust
/// # use cursive_core as cursive;
/// use cursive::traits::Nameable;
/// use cursive::views::Checkbox;
///
/// let checkbox = Checkbox::new().checked().with_name("check");
/// ```
pub struct Checkbox {
    checked: bool,
    enabled: bool,

    on_change: Option<Arc<Callback>>,

    label: StyledString,
}

new_default!(Checkbox);

impl Checkbox {
    impl_enabled!(self.enabled);

    /// Creates a new, unlabelled, unchecked checkbox.
    pub fn new() -> Self {
        Checkbox {
            checked: false,
            enabled: true,
            on_change: None,
            label: StyledString::new(),
        }
    }

    /// Creates a new, labelled, unchecked checkbox.
    pub fn labelled<S: Into<StyledString>>(label: S) -> Self {
        Checkbox {
            checked: false,
            enabled: true,
            on_change: None,
            label: label.into(),
        }
    }

    /// Sets a callback to be used when the state changes.
    #[crate::callback_helpers]
    pub fn set_on_change<F: 'static + Fn(&mut Cursive, bool) + Send + Sync>(
        &mut self,
        on_change: F,
    ) {
        self.on_change = Some(Arc::new(on_change));
    }

    /// Sets a callback to be used when the state changes.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_change<F: 'static + Fn(&mut Cursive, bool) + Send + Sync>(
        self,
        on_change: F,
    ) -> Self {
        self.with(|s| s.set_on_change(on_change))
    }

    /// Toggles the checkbox state.
    pub fn toggle(&mut self) -> EventResult {
        let checked = !self.checked;
        self.set_checked(checked)
    }

    /// Check the checkbox.
    pub fn check(&mut self) -> EventResult {
        self.set_checked(true)
    }

    /// Check the checkbox.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn checked(self) -> Self {
        self.with(|s| {
            s.check();
        })
    }

    /// Returns `true` if the checkbox is checked.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::Checkbox;
    ///
    /// let mut checkbox = Checkbox::new().checked();
    /// assert!(checkbox.is_checked());
    ///
    /// checkbox.uncheck();
    /// assert!(!checkbox.is_checked());
    /// ```
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Uncheck the checkbox.
    pub fn uncheck(&mut self) -> EventResult {
        self.set_checked(false)
    }

    /// Uncheck the checkbox.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn unchecked(self) -> Self {
        self.with(|s| {
            s.uncheck();
        })
    }

    /// Sets the checkbox state.
    pub fn set_checked(&mut self, checked: bool) -> EventResult {
        self.checked = checked;
        if let Some(ref on_change) = self.on_change {
            let on_change = Arc::clone(on_change);
            EventResult::with_cb(move |s| on_change(s, checked))
        } else {
            EventResult::Consumed(None)
        }
    }

    /// Set the checkbox state.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_checked(self, is_checked: bool) -> Self {
        self.with(|s| {
            s.set_checked(is_checked);
        })
    }

    fn draw_internal(&self, printer: &Printer) {
        printer.print((0, 0), "[ ]");
        if self.checked {
            printer.print((1, 0), "X");
        }

        if !self.label.is_empty() {
            // We want the space to be highlighted if focused
            printer.print((3, 0), " ");
            printer.print_styled((4, 0), &self.label);
        }
    }
}

impl View for Checkbox {
    fn required_size(&mut self, _: Vec2) -> Vec2 {
        if self.label.is_empty() {
            Vec2::new(3, 1)
        } else {
            Vec2::new(3 + 1 + self.label.width(), 1)
        }
    }

    fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
        self.enabled.then(EventResult::consumed).ok_or(CannotFocus)
    }

    fn draw(&self, printer: &Printer) {
        if self.enabled && printer.enabled {
            printer.with_selection(printer.focused, |printer| self.draw_internal(printer));
        } else {
            printer.with_style(PaletteStyle::Secondary, |printer| {
                self.draw_internal(printer)
            });
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }
        match event {
            Event::Key(Key::Enter) | Event::Char(' ') => self.toggle(),
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset, (3, 1)) => self.toggle(),
            _ => EventResult::Ignored,
        }
    }
}

#[crate::blueprint(Checkbox::new())]
struct Blueprint {
    on_change: Option<_>,

    checked: Option<bool>,
    enabled: Option<bool>,
}
