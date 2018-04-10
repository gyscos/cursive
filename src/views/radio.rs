use {Printer, With};
use direction::Direction;
use event::{Event, EventResult, Key, MouseButton, MouseEvent};
use std::cell::RefCell;
use std::rc::Rc;
use theme::ColorStyle;
use vec::Vec2;
use view::View;

struct SharedState<T> {
    selection: usize,
    values: Vec<Rc<T>>,
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
///
/// [`RadioButton`]: struct.RadioButton.html
#[derive(Clone)]
pub struct RadioGroup<T> {
    // Given to every child button
    state: Rc<RefCell<SharedState<T>>>,
}

impl<T> Default for RadioGroup<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> RadioGroup<T> {
    /// Creates an empty group for radio buttons.
    pub fn new() -> Self {
        RadioGroup {
            state: Rc::new(RefCell::new(SharedState {
                selection: 0,
                values: Vec::new(),
            })),
        }
    }

    /// Adds a new button to the group.
    ///
    /// The button will display `label` next to it, and will embed `value`.
    pub fn button<S: Into<String>>(
        &mut self, value: T, label: S
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
    pub fn selection(&self) -> Rc<T> {
        self.state.borrow().selection()
    }
}

impl RadioGroup<String> {
    /// Adds a button, using the label itself as value.
    pub fn button_str<S: Into<String>>(
        &mut self, text: S
    ) -> RadioButton<String> {
        let text = text.into();
        self.button(text.clone(), text)
    }
}

/// Variant of `Checkbox` arranged in group.
///
/// `RadioButton`s are managed by a [`RadioGroup`]. A single group can contain
/// several radio buttons, but only one button per group can be active at a
/// time.
///
/// `RadioButton`s are not created directly, but through
/// [`RadioGroup::button()`].
///
/// [`RadioGroup`]: struct.RadioGroup.html
/// [`RadioGroup::button()`]: struct.RadioGroup.html#method.button
pub struct RadioButton<T> {
    state: Rc<RefCell<SharedState<T>>>,
    id: usize,
    enabled: bool,
    label: String,
}

impl<T> RadioButton<T> {
    impl_enabled!(self.enabled);

    fn new(
        state: Rc<RefCell<SharedState<T>>>, id: usize, label: String
    ) -> Self {
        RadioButton {
            state,
            id,
            enabled: true,
            label,
        }
    }

    /// Returns `true` if this button is selected.
    pub fn is_selected(&self) -> bool {
        self.state.borrow().selection == self.id
    }

    /// Selects this button, un-selecting any other in the same group.
    pub fn select(&mut self) {
        self.state.borrow_mut().selection = self.id;
    }

    /// Selects this button, un-selecting any other in the same group.
    ///
    /// Chainable variant.
    pub fn selected(self) -> Self {
        self.with(Self::select)
    }

    fn draw_internal(&self, printer: &Printer) {
        printer.print((0, 0), "( )");
        if self.is_selected() {
            printer.print((1, 0), "X");
        }

        if !self.label.is_empty() {
            // We want the space to be highlighted if focused
            printer.print((3, 0), " ");
            printer.print((4, 0), &self.label);
        }
    }

    fn req_size(&self) -> Vec2 {
        if self.label.is_empty() {
            Vec2::new(3, 1)
        } else {
            Vec2::new(3 + 1 + self.label.len(), 1)
        }
    }
}

impl<T: 'static> View for RadioButton<T> {
    fn required_size(&mut self, _: Vec2) -> Vec2 {
        self.req_size()
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled
    }

    fn draw(&self, printer: &Printer) {
        if self.enabled {
            printer.with_selection(printer.focused, |printer| {
                self.draw_internal(printer)
            });
        } else {
            printer.with_color(ColorStyle::secondary(), |printer| {
                self.draw_internal(printer)
            });
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Enter) | Event::Char(' ') => {
                self.select();
                EventResult::Consumed(None)
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset, self.req_size()) =>
            {
                self.select();
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored,
        }
    }
}
