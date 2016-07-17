use With;
use Cursive;
use Printer;
use vec::Vec2;
use view::View;
use event::{Event, EventResult, Key};
use direction::Direction;

use std::rc::Rc;


/// Checkable box.
pub struct Checkbox {
    checked: bool,

    on_change: Option<Rc<Fn(&mut Cursive, bool)>>,
}

new_default!(Checkbox);

impl Checkbox {
    /// Creates a new, unchecked checkbox.
    pub fn new() -> Self {
        Checkbox {
            checked: false,
            on_change: None,
        }
    }

    /// Sets a callback to be used when the state changes.
    pub fn set_on_change<F: 'static + Fn(&mut Cursive, bool)>(&mut self, on_change: F) {
        self.on_change = Some(Rc::new(on_change));
    }

    /// Sets a callback to be used when the state changes.
    ///
    /// Chainable variant.
    pub fn on_change<F: 'static + Fn(&mut Cursive, bool)>(self, on_change: F) -> Self {
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
    pub fn checked(self) -> Self {
        self.with(|s| {s.check();})
    }

    /// Returns `true` if the checkbox is checked.
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
    pub fn unchecked(self) -> Self {
        self.with(|s| { s.uncheck(); })
    }

    /// Sets the checkbox state.
    pub fn set_checked(&mut self, checked: bool) -> EventResult {
        self.checked = checked;
        if let Some(ref on_change) = self.on_change {
            let on_change = on_change.clone();
            EventResult::with_cb(move |s| on_change(s, checked))
        } else {
            EventResult::Consumed(None)
        }
    }
}

impl View for Checkbox {
    fn get_min_size(&mut self, _: Vec2) -> Vec2 {
        Vec2::new(3, 1)
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        true
    }

    fn draw(&self, printer: &Printer) {
        printer.with_selection(printer.focused, |printer| {
            printer.print((0, 0), "[ ]");
            if self.checked {
                printer.print((1, 0), "X");
            }
        });

    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Enter) |
            Event::Char(' ') => self.toggle(),
            _ => EventResult::Ignored,
        }
    }
}
