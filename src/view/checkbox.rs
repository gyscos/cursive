use With;
use Printer;
use vec::Vec2;
use view::View;
use event::{Event, EventResult, Key};
use direction::Direction;


/// Checkable box.
#[derive(Debug)]
pub struct Checkbox {
    checked: bool,
}

new_default!(Checkbox);

impl Checkbox {
    /// Creates a new, unchecked checkbox.
    pub fn new() -> Self {
        Checkbox { checked: false }
    }

    /// Toggles the checkbox state.
    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }

    /// Check the checkbox.
    pub fn check(&mut self) {
        self.checked = true;
    }

    /// Check the checkbox.
    ///
    /// Chainable variant.
    pub fn checked(self) -> Self {
        self.with(Self::check)
    }

    /// Returns `true` if the checkbox is checked.
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Uncheck the checkbox.
    pub fn uncheck(&mut self) {
        self.checked = false;
    }

    /// Uncheck the checkbox.
    ///
    /// Chainable variant.
    pub fn unchecked(self) -> Self {
        self.with(Self::uncheck)
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
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }
}
