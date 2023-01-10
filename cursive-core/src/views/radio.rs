use crate::{
    direction::Direction,
    event::{Event, EventResult, Key, MouseButton, MouseEvent},
    theme::PaletteStyle,
    utils::markup::StyledString,
    view::{CannotFocus, View},
    Cursive, Printer, Vec2,
};
use std::any::Any;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

type Callback<T> = dyn Fn(&mut Cursive, &T);

// Maps keys (strings) to RadioGroup<T> (wrapped in a Box<Any>).
// So with a key: &str and a concrete T, we can get the matching `RadioGroup<T>`, or create one.
//
// TODO: should we use thread local instead of a static mutex?
thread_local! {
    static GROUPS: RefCell<BTreeMap<(String, TypeId), Box<dyn Any>>> =
        RefCell::new(BTreeMap::new());
}

struct SharedState<T> {
    selection: usize,
    values: Vec<Rc<T>>,

    on_change: Option<Rc<Callback<T>>>,
}

impl<T> SharedState<T> {
    pub fn selection(&self) -> Rc<T> {
        Rc::clone(&self.values[self.selection])
    }
}

/// Group to coordinate multiple radio buttons.
///
/// A `RadioGroup` is used to create and manage [`RadioButton`]s.
///
/// A `RadioGroup` can be cloned; it will keep pointing to the same group.
pub struct RadioGroup<T> {
    // Given to every child button
    state: Rc<RefCell<SharedState<T>>>,
}

// We have to manually implement Clone.
// Using derive(Clone) would add am unwanted `T: Clone` where-clause.
impl<T> Clone for RadioGroup<T> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl<T: 'static> Default for RadioGroup<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> RadioGroup<T> {
    /// Creates an empty group for radio buttons.
    pub fn new() -> Self {
        RadioGroup {
            state: Rc::new(RefCell::new(SharedState {
                selection: 0,
                values: Vec::new(),
                on_change: None,
            })),
        }
    }

    /// Run a closure on a radio group from a global pool.
    ///
    /// If none exist with the given type `T` and `key`, a new one will be created.
    pub fn with_global<F, R>(key: &str, f: F) -> R
    where
        F: FnOnce(&mut RadioGroup<T>) -> R,
    {
        let type_id = TypeId::of::<T>();

        GROUPS.with(|groups| {
            let mut groups = groups.borrow_mut();

            let group = groups
                .entry((key.to_string(), type_id))
                .or_insert(Box::new(RadioGroup::<T>::new()));

            // Because we key by TypeId we _know_ it'll be the correct type.
            let group = group.downcast_mut().unwrap();

            f(group)
        })
    }

    /// Adds a new button to the group.
    ///
    /// The button will display `label` next to it, and will embed `value`.
    pub fn button<S: Into<StyledString>>(
        &mut self,
        value: T,
        label: S,
    ) -> RadioButton<T> {
        let count = self.state.borrow().values.len();
        self.state.borrow_mut().values.push(Rc::new(value));
        RadioButton::new(Rc::clone(&self.state), count, label.into())
    }

    /// Returns the id of the selected button.
    ///
    /// Buttons are indexed in the order they are created, starting from 0.
    pub fn selected_id(&self) -> usize {
        self.state.borrow().selection
    }

    /// Returns the value associated with the selected button.
    ///
    /// # Panics
    ///
    /// If the group is empty (no button).
    pub fn selection(&self) -> Rc<T> {
        self.state.borrow().selection()
    }

    /// Sets a callback to be used when the selection changes.
    pub fn set_on_change<F: 'static + Fn(&mut Cursive, &T)>(
        &mut self,
        on_change: F,
    ) {
        self.state.borrow_mut().on_change = Some(Rc::new(on_change));
    }

    /// Sets a callback to be used when the selection changes.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_change<F: 'static + Fn(&mut Cursive, &T)>(
        self,
        on_change: F,
    ) -> Self {
        // We need .with for the thread local, so we can't import the With trait...
        crate::With::with(self, |s| s.set_on_change(on_change))
    }
}

impl RadioGroup<String> {
    /// Adds a button, using the label itself as value.
    pub fn button_str<S: Into<String>>(
        &mut self,
        text: S,
    ) -> RadioButton<String> {
        let text = text.into();
        self.button(text.clone(), text)
    }
}

impl RadioButton<String> {
    /// Create a new button on the global group pool.
    ///
    /// Uses the label itself as value.
    ///
    /// If no group exist for the given `key` and type `T`, one will be created.
    pub fn global_str<S: Into<String>>(key: &str, text: S) -> Self {
        RadioGroup::with_global(key, move |group| group.button_str(text))
    }
}
/// Variant of `Checkbox` arranged in group.
///
/// `RadioButton`s are managed by a [`RadioGroup`]. A single group can contain
/// several radio buttons, but only one button per group can be active at a
/// time.
///
/// `RadioButton`s are not created directly, but through
/// [`RadioGroup::button`].
pub struct RadioButton<T> {
    state: Rc<RefCell<SharedState<T>>>,
    id: usize,
    enabled: bool,
    label: StyledString,
}

impl<T: 'static> RadioButton<T> {
    impl_enabled!(self.enabled);

    fn new(
        state: Rc<RefCell<SharedState<T>>>,
        id: usize,
        label: StyledString,
    ) -> Self {
        RadioButton {
            state,
            id,
            enabled: true,
            label,
        }
    }

    /// Create a new button on the global group pool.
    ///
    /// If no group exist for the given `key` and type `T`, one will be created.
    pub fn global<S: Into<StyledString>>(
        key: &str,
        value: T,
        label: S,
    ) -> Self {
        RadioGroup::with_global(key, move |group| group.button(value, label))
    }

    /// Returns `true` if this button is selected.
    pub fn is_selected(&self) -> bool {
        self.state.borrow().selection == self.id
    }

    /// Selects this button, un-selecting any other in the same group.
    pub fn select(&mut self) -> EventResult {
        let mut state = self.state.borrow_mut();
        state.selection = self.id;
        if let Some(ref on_change) = state.on_change {
            let on_change = Rc::clone(on_change);
            let value = state.selection();
            EventResult::with_cb(move |s| on_change(s, &value))
        } else {
            EventResult::Consumed(None)
        }
    }

    /// Selects this button, un-selecting any other in the same group.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn selected(self) -> Self {
        crate::With::with(self, |s| {
            // Ignore the potential callback here
            s.select();
        })
    }

    fn draw_internal(&self, printer: &Printer) {
        printer.print((0, 0), "( )");
        if self.is_selected() {
            printer.print((1, 0), "X");
        }

        if !self.label.is_empty() {
            // We want the space to be highlighted if focused
            printer.print((3, 0), " ");
            printer.print_styled((4, 0), &self.label);
        }
    }

    fn req_size(&self) -> Vec2 {
        if self.label.is_empty() {
            Vec2::new(3, 1)
        } else {
            Vec2::new(3 + 1 + self.label.width(), 1)
        }
    }
}

impl<T: 'static> View for RadioButton<T> {
    fn required_size(&mut self, _: Vec2) -> Vec2 {
        self.req_size()
    }

    fn take_focus(
        &mut self,
        _: Direction,
    ) -> Result<EventResult, CannotFocus> {
        self.enabled.then(EventResult::consumed).ok_or(CannotFocus)
    }

    fn draw(&self, printer: &Printer) {
        if self.enabled && printer.enabled {
            printer.with_selection(printer.focused, |printer| {
                self.draw_internal(printer)
            });
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
            Event::Key(Key::Enter) | Event::Char(' ') => self.select(),
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset, self.req_size()) => {
                self.select()
            }
            _ => EventResult::Ignored,
        }
    }
}
