use crate::{
    direction::{Direction, Orientation},
    event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent},
    style::PaletteStyle,
    view::{CannotFocus, View},
    Cursive, Printer, Vec2, With,
};
use std::sync::Arc;

type SliderCallback = dyn Fn(&mut Cursive, usize) + Send + Sync;

/// A horizontal or vertical slider.
///
/// # Examples
///
/// ```
/// use cursive_core::views::{Dialog, SliderView};
///
/// let slider_view = SliderView::horizontal(10)
///     .on_change(|s, n| {
///         if n == 5 {
///             s.add_layer(Dialog::info("5! Pick 5!"));
///         }
///     })
///     .on_enter(|s, n| match n {
///         5 => s.add_layer(Dialog::info("You did it!")),
///         n => s.add_layer(Dialog::info(format!("Why {}? Why not 5?", n))),
///     });
/// ```
pub struct SliderView {
    orientation: Orientation,
    on_change: Option<Arc<SliderCallback>>,
    on_enter: Option<Arc<SliderCallback>>,
    value: usize,
    max_value: usize,
    dragging: bool,
}

impl SliderView {
    /// Creates a new `SliderView` in the given orientation.
    ///
    /// The view will have a fixed length of `max_value`,
    /// with one tick per block.
    ///
    /// The actual range of values for this slider is `[0, max_value - 1]`.
    pub fn new(orientation: Orientation, max_value: usize) -> Self {
        SliderView {
            orientation,
            value: 0,
            max_value,
            on_change: None,
            on_enter: None,
            dragging: false,
        }
    }

    /// Creates a new vertical `SliderView`.
    pub fn vertical(max_value: usize) -> Self {
        Self::new(Orientation::Vertical, max_value)
    }

    /// Creates a new horizontal `SliderView`.
    pub fn horizontal(max_value: usize) -> Self {
        Self::new(Orientation::Horizontal, max_value)
    }

    /// Sets the current value.
    ///
    /// Returns an event result with a possible callback,
    /// if `on_change` was set..
    pub fn set_value(&mut self, value: usize) -> EventResult {
        self.value = value;
        self.get_change_result()
    }

    /// Sets the current value.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn value(self, value: usize) -> Self {
        self.with(|s| {
            s.set_value(value);
        })
    }

    /// Gets the current value.
    pub fn get_value(&self) -> usize {
        self.value
    }

    /// Gets the max value.
    pub fn get_max_value(&self) -> usize {
        self.max_value
    }

    /// Sets a callback to be called when the slider is moved.
    #[crate::callback_helpers]
    pub fn set_on_change<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, usize) + 'static + Send + Sync,
    {
        self.on_change = Some(Arc::new(callback));
    }

    /// Sets a callback to be called when the slider is moved.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_change<F>(self, callback: F) -> Self
    where
        F: Fn(&mut Cursive, usize) + 'static + Send + Sync,
    {
        self.with(|s| s.set_on_change(callback))
    }

    /// Sets a callback to be called when the `<Enter>` key is pressed.
    #[crate::callback_helpers]
    pub fn set_on_enter<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, usize) + 'static + Send + Sync,
    {
        self.on_enter = Some(Arc::new(callback));
    }

    /// Sets a callback to be called when the `<Enter>` key is pressed.
    #[must_use]
    pub fn on_enter<F>(self, callback: F) -> Self
    where
        F: Fn(&mut Cursive, usize) + 'static + Send + Sync,
    {
        self.with(|s| s.set_on_enter(callback))
    }

    fn get_change_result(&self) -> EventResult {
        EventResult::Consumed(self.on_change.clone().map(|cb| {
            let value = self.value;
            Callback::from_fn(move |s| {
                cb(s, value);
            })
        }))
    }

    fn slide_plus(&mut self) -> EventResult {
        if self.value + 1 < self.max_value {
            self.value += 1;
            self.get_change_result()
        } else {
            EventResult::Ignored
        }
    }

    fn slide_minus(&mut self) -> EventResult {
        if self.value > 0 {
            self.value -= 1;
            self.get_change_result()
        } else {
            EventResult::Ignored
        }
    }

    fn req_size(&self) -> Vec2 {
        self.orientation.make_vec(self.max_value, 1)
    }
}

impl View for SliderView {
    fn draw(&self, printer: &Printer) {
        match self.orientation {
            Orientation::Vertical => printer.print_vline((0, 0), self.max_value, "|"),
            Orientation::Horizontal => printer.print_hline((0, 0), self.max_value, "-"),
        }

        let style = if printer.focused {
            PaletteStyle::Highlight
        } else {
            PaletteStyle::HighlightInactive
        };

        printer.with_style(style, |printer| {
            printer.print(self.orientation.make_vec(self.value, 0), " ");
        });
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        self.req_size()
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Left) if self.orientation == Orientation::Horizontal => {
                self.slide_minus()
            }
            Event::Key(Key::Right) if self.orientation == Orientation::Horizontal => {
                self.slide_plus()
            }
            Event::Key(Key::Up) if self.orientation == Orientation::Vertical => self.slide_minus(),
            Event::Key(Key::Down) if self.orientation == Orientation::Vertical => self.slide_plus(),
            Event::Key(Key::Enter) if self.on_enter.is_some() => {
                let value = self.value;
                let cb = self.on_enter.clone().unwrap();
                EventResult::with_cb(move |s| {
                    cb(s, value);
                })
            }
            Event::Mouse {
                event: MouseEvent::Hold(MouseButton::Left),
                position,
                offset,
            } if self.dragging => {
                let position = position.saturating_sub(offset);
                let position = self.orientation.get(&position);
                let position = ::std::cmp::min(position, self.max_value.saturating_sub(1));
                self.value = position;
                self.get_change_result()
            }
            Event::Mouse {
                event: MouseEvent::Press(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset, self.req_size()) => {
                if let Some(position) = position.checked_sub(offset) {
                    self.dragging = true;
                    self.value = self.orientation.get(&position);
                }
                self.get_change_result()
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                ..
            } => {
                self.dragging = false;
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }

    fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
        Ok(EventResult::Consumed(None))
    }
}

// TODO: Rename the view itself as Slider to match the config?
#[crate::blueprint(SliderView::new(orientation, max_value))]
struct Blueprint {
    orientation: Orientation,
    max_value: usize,

    on_change: Option<_>,
    on_enter: Option<_>,
}
