use {Cursive, Printer};

use With;
use direction::{Direction, Orientation};
use event::{Callback, Event, EventResult, Key};
use std::rc::Rc;
use theme::ColorStyle;
use vec::Vec2;
use view::View;

/// A horizontal or vertical slider.
pub struct SliderView {
    orientation: Orientation,
    on_change: Option<Rc<Fn(&mut Cursive, usize)>>,
    on_enter: Option<Rc<Fn(&mut Cursive, usize)>>,
    value: usize,
    max_value: usize,
}

impl SliderView {
    /// Creates a new `SliderView` in the given orientation.
    ///
    /// The view will have a fixed length of `max_value`,
    /// with one tick per block.
    pub fn new(orientation: Orientation, max_value: usize) -> Self {
        SliderView {
            orientation: orientation,
            value: 0,
            max_value: max_value,
            on_change: None,
            on_enter: None,
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
    pub fn value(self, value: usize) -> Self {
        self.with(|s| {
            s.set_value(value);
        })
    }

    /// Sets a callback to be called when the slider is moved.
    pub fn on_change<F>(mut self, callback: F) -> Self
        where F: Fn(&mut Cursive, usize) + 'static
    {
        self.on_change = Some(Rc::new(callback));
        self
    }

    /// Sets a callback to be called when the <Enter> key is pressed.
    pub fn on_enter<F>(mut self, callback: F) -> Self
        where F: Fn(&mut Cursive, usize) + 'static
    {
        self.on_enter = Some(Rc::new(callback));
        self
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
}

impl View for SliderView {
    fn draw(&self, printer: &Printer) {
        match self.orientation {
            Orientation::Vertical => {
                printer.print_vline((0, 0), self.max_value, "|")
            }
            Orientation::Horizontal => {
                printer.print_hline((0, 0), self.max_value, "-")
            }
        }

        let color = if printer.focused {
            ColorStyle::Highlight
        } else {
            ColorStyle::HighlightInactive
        };
        printer.with_color(color, |printer| {
            printer.print(self.orientation.make_vec(self.value, 0), " ");
        });
    }

    fn get_min_size(&mut self, _: Vec2) -> Vec2 {
        self.orientation.make_vec(self.max_value, 1)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Left) if self.orientation ==
                                     Orientation::Horizontal => {
                self.slide_minus()
            }
            Event::Key(Key::Right) if self.orientation ==
                                      Orientation::Horizontal => {
                self.slide_plus()
            }
            Event::Key(Key::Up) if self.orientation ==
                                   Orientation::Vertical => self.slide_minus(),
            Event::Key(Key::Down) if self.orientation ==
                                     Orientation::Vertical => {
                self.slide_plus()
            }
            Event::Key(Key::Enter) if self.on_enter.is_some() => {
                let value = self.value;
                let cb = self.on_enter.clone().unwrap();
                EventResult::with_cb(move |s| {
                    cb(s, value);
                })
            }
            _ => EventResult::Ignored,
        }
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        true
    }
}
