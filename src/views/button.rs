

use {Cursive, Printer, With};
use align::HAlign;
use direction::Direction;
use event::*;
use theme::ColorStyle;
use unicode_width::UnicodeWidthStr;
use vec::Vec2;
use view::View;

/// Simple text label with a callback when <Enter> is pressed.
///
/// A button shows its content in a single line and has a fixed size.
///
/// # Examples
///
/// ```
/// # use cursive::views::Button;
/// let quit_button = Button::new("Quit", |s| s.quit());
/// ```
pub struct Button {
    label: String,
    callback: Callback,
    enabled: bool,
}

impl Button {
    /// Creates a new button with the given content and callback.
    pub fn new<F, S: Into<String>>(label: S, cb: F) -> Self
    where
        F: Fn(&mut Cursive) + 'static,
    {
        Button {
            label: label.into(),
            callback: Callback::from_fn(cb),
            enabled: true,
        }
    }

    /// Sets the function to be called when the button is pressed.
    ///
    /// Replaces the previous callback.
    pub fn set_callback<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive) + 'static,
    {
        self.callback = Callback::from_fn(cb);
    }

    /// Disables this view.
    ///
    /// A disabled view cannot be selected.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Disables this view.
    ///
    /// Chainable variant.
    pub fn disabled(self) -> Self {
        self.with(Self::disable)
    }

    /// Re-enables this view.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Enable or disable this view.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns `true` if this view is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn req_size(&self) -> Vec2 {
        Vec2::new(2 + self.label.width(), 1)
    }
}

impl View for Button {
    fn draw(&self, printer: &Printer) {
        if printer.size.x == 0 {
            return;
        }

        let style = if !self.enabled {
            ColorStyle::Secondary
        } else if !printer.focused {
            ColorStyle::Primary
        } else {
            ColorStyle::Highlight
        };

        let offset =
            HAlign::Center.get_offset(self.label.len() + 2, printer.size.x);

        printer.with_color(style, |printer| {
            printer.print((offset, 0), &format!("<{}>", self.label));
        });
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        // Meh. Fixed size we are.
        self.req_size()
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // eprintln!("{:?}", event);
        // eprintln!("{:?}", self.req_size());
        match event {
            // 10 is the ascii code for '\n', that is the return key
            Event::Key(Key::Enter) => {
                EventResult::Consumed(Some(self.callback.clone()))
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset, self.req_size()) =>
            {
                EventResult::Consumed(Some(self.callback.clone()))
            }
            _ => EventResult::Ignored,
        }
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled
    }
}
