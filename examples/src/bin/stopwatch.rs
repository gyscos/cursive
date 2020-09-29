//! A simple stopwatch is implemented in this example.
//! Press "Space" to start/pause/resume the stopwatch. Press "Enter" to stop and
//! get all data: moments at which the stopwatch is started/resumed, moments at which
//! the stopwatch is paused/stopped; elapsed time.
//! You may also want to explore the crate `clock-cli` (https://github.com/TianyiShi2001/cl!ock-cli-rs),
//! which aims to implement a fully-fledged clock with stopwatch, countdown
//! timer, and possibly more functionalities.

use chrono::{DateTime, Duration, Local};
use cursive::{
    event::{Callback, Event, EventResult, Key},
    traits::*,
    view::View,
    views::Dialog,
    Cursive, Printer, Vec2, With,
};
use std::rc::Rc;

fn main() {
    let mut siv = cursive::default();
    let stopwatch = StopWatchView::new();
    siv.add_layer(
        stopwatch
            // On stop, get all data and summarize them in an info box.
            .on_stop(|s: &mut Cursive, stopwatch: StopWatch| {
                s.add_layer(Dialog::info(format!(
                    "Elapsed: {}",
                    stopwatch.elapsed.pretty()
                )))
            }),
    );
    siv.add_layer(Dialog::info(
        "Press 'Space' to start/pause/resume the stopwatch\nPress 'Enter' to stop",
    ));
    // the stopwatch is redrawn 15 times per second
    siv.set_fps(15);
    siv.run();
}

#[derive(Clone, Debug)]
pub struct StopWatch {
    // These data might be useful to the user
    pub elapsed: Duration, // total elapsed time
    pub last_update: DateTime<Local>,
    paused: bool,
}

impl StopWatch {
    /// Returns a stopwatch that is reset to zero
    pub fn new() -> Self {
        Self {
            elapsed: Duration::zero(),
            last_update: Local::now(),
            paused: true, // stopped by default; start by explicitly calling `.resume()`
        }
    }

    fn pause(&mut self) {
        let moment = Local::now();
        self.elapsed = self.elapsed + (moment - self.last_update);
        self.paused = true;
    }
    fn resume(&mut self) {
        self.last_update = Local::now();
        self.paused = false;
    }
    fn pause_or_resume(&mut self) {
        if self.paused {
            self.resume();
        } else {
            self.pause();
        }
    }
    /// Read the total time elapsed
    fn read(&self) -> Duration {
        if self.paused {
            self.elapsed
        } else {
            self.elapsed + (Local::now() - self.last_update)
        }
    }
}

/// Separating the `StopWatch` 'core' and the `StopWatchView` improves reusability
/// and flexibility. The user may implement their own `View`s, i.e. layouts, based
/// on the same `StopWatch` logic.
pub struct StopWatchView {
    stopwatch: StopWatch,
    on_stop: Option<Rc<dyn Fn(&mut Cursive, StopWatch)>>,
}

impl StopWatchView {
    pub fn new() -> Self {
        Self {
            stopwatch: StopWatch::new(),
            on_stop: None,
        }
    }

    /// Sets a callback to be used when `<Enter>` is pressed.
    ///
    /// The elapsed time will be given to the callback.
    ///
    /// See also cursive::views::select_view::SelectView::set_on_submit
    pub fn set_on_stop<F, R>(&mut self, cb: F)
    where
        F: 'static + Fn(&mut Cursive, StopWatch) -> R,
    {
        self.on_stop = Some(Rc::new(move |s, t| {
            cb(s, t);
        }));
    }

    pub fn on_stop<F, R>(self, cb: F) -> Self
    where
        F: 'static + Fn(&mut Cursive, StopWatch) -> R,
    {
        self.with(|s| s.set_on_stop(cb))
    }

    fn stop(&mut self) -> EventResult {
        if !self.stopwatch.paused {
            self.stopwatch.pause();
        }
        // get the ownership of the fresh data from self.stopwatch, and replace
        // self.stopwatch with a new one (i.e. reset to zero)
        let stopwatch =
            std::mem::replace(&mut self.stopwatch, StopWatch::new());
        if self.on_stop.is_some() {
            let cb = self.on_stop.clone().unwrap();
            EventResult::Consumed(Some(Callback::from_fn_once(move |s| {
                cb(s, stopwatch)
            })))
        } else {
            EventResult::Consumed(None)
        }
    }
}
impl View for StopWatchView {
    fn draw(&self, printer: &Printer) {
        printer.print((0, 0), &self.stopwatch.read().pretty());
    }

    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        Vec2::new(12, 1) // columns, rows (width, height)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            // pause/resume the stopwatch when pressing "Space"
            Event::Char(' ') => {
                self.stopwatch.pause_or_resume();
            }
            // stop (reset) the stopwatch when pressing "Enter"
            Event::Key(Key::Enter) => {
                return self.stop();
            }
            _ => return EventResult::Ignored,
        }
        EventResult::Consumed(None)
    }
}

pub trait PrettyDuration {
    fn pretty(&self) -> String;
}
impl PrettyDuration for Duration {
    /// Pretty-prints a chrono::Duration in the form `HH:MM:SS.xxx`
    /// A custom trait is used because `std::fmt::Diaplay` cannot be implemented
    /// for a struct coming from another external crate, due to the orphan rule
    fn pretty(&self) -> String {
        let s = self.num_seconds();
        let ms = self.num_milliseconds() - 1000 * s;
        let (h, s) = (s / 3600, s % 3600);
        let (m, s) = (s / 60, s % 60);
        format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
    }
}
