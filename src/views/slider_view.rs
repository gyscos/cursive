use crate::direction::{Direction, Orientation};
use crate::event::{
    Callback, Event, EventResult, Key, MouseButton, MouseEvent,
};
use crate::theme::ColorStyle;
use crate::vec::Vec2;
use crate::view::View;
use crate::With;
use crate::{Cursive, Printer};
use std::rc::Rc;

/// A horizontal or vertical slider.
pub struct SliderView {
    orientation: Orientation,
    on_change: Option<Rc<dyn Fn(&mut Cursive, usize)>>,
    on_enter: Option<Rc<dyn Fn(&mut Cursive, usize)>>,
    value: usize,
    max_value: usize,
    dragging: bool,
}

impl SliderView {
    /// Creates a new `SliderView` in the given orientation.
    ///
    /// The view will have a fixed length of `max_value`,
    /// with one tick per block.
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
    pub fn value(self, value: usize) -> Self {
        self.with(|s| {
            s.set_value(value);
        })
    }

    /// Sets a callback to be called when the slider is moved.
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Cursive, usize) + 'static,
    {
        self.on_change = Some(Rc::new(callback));
        self
    }

    /// Sets a callback to be called when the <Enter> key is pressed.
    pub fn on_enter<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Cursive, usize) + 'static,
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

    fn req_size(&self) -> Vec2 {
        self.orientation.make_vec(self.max_value, 1)
    }
}

impl View for SliderView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        match self.orientation {
            Orientation::Vertical => {
                printer.print_vline((0, 0), self.max_value, "|")
            }
            Orientation::Horizontal => {
                printer.print_hline((0, 0), self.max_value, "-")
            }
        }

        let color = if printer.focused {
            ColorStyle::highlight()
        } else {
            ColorStyle::highlight_inactive()
        };
        printer.with_color(color, |printer| {
            printer.print(self.orientation.make_vec(self.value, 0), " ");
        });
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        self.req_size()
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Left)
                if self.orientation == Orientation::Horizontal =>
            {
                self.slide_minus()
            }
            Event::Key(Key::Right)
                if self.orientation == Orientation::Horizontal =>
            {
                self.slide_plus()
            }
            Event::Key(Key::Up)
                if self.orientation == Orientation::Vertical =>
            {
                self.slide_minus()
            }
            Event::Key(Key::Down)
                if self.orientation == Orientation::Vertical =>
            {
                self.slide_plus()
            }
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
                let position = ::std::cmp::min(
                    position,
                    self.max_value.saturating_sub(1),
                );
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

    fn take_focus(&mut self, _: Direction) -> bool {
        true
    }
}
