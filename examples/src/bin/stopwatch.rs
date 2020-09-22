use cursive::{traits::*, views::Dialog, Cursive};

fn main() {
    let mut siv = cursive::default();
    let stopwatch = StopWatch::StopWatchView::new();
    siv.add_layer(
        stopwatch
            .with_laps(8)
            .on_stop(|s: &mut Cursive, elapsed| {
                s.add_layer(Dialog::info(format!(
                    "Elapsed time: {}",
                    elapsed.pretty()
                )))
            })
            .with_name("stopwatch"),
    );
    siv.add_layer(Dialog::info(
        "Press 'Space' to start/pause/resume the stopwatch\nPress 'l' to record lap time\nPress 'Enter' to stop",
    ));
    siv.set_fps(15);
    siv.run();
}

mod StopWatch {
    use super::PrettyDuration;
    use chrono::{DateTime, Duration, Local};
    use cursive::{
        event::{Callback, Event, EventResult, Key},
        view::View,
        Cursive, Printer, Vec2, With,
    };
    use std::rc::Rc;

    /// A stopwatch that mimics iOS's stopwatch
    ///
    /// ```ignore
    ///                  lap    lap          lap
    /// start       start |      |     start  |
    ///   o--------x   o-----------x      o-----------x
    ///          pause           pause            pause(end)
    /// ```
    pub struct StopWatch {
        // These data *might* be useful to the user
        pub elapsed: Duration,     // total elapsed time
        pub lap_elapsed: Duration, // elapsed time of the current lap
        pub pause_moments: Vec<DateTime<Local>>, // moments at which the stopwatch is paused
        pub start_moments: Vec<DateTime<Local>>, // moments at which the stopwatch resumes
        pub lap_moments: Vec<DateTime<Local>>, // moments at which a lap time is read
        pub laps: Vec<Duration>,               // lap times
        paused: bool,
    }

    impl StopWatch {
        /// Returns stopwatch reset to zero
        pub fn new() -> Self {
            Self {
                elapsed: Duration::zero(),
                lap_elapsed: Duration::zero(),
                start_moments: Vec::new(),
                pause_moments: Vec::new(),
                lap_moments: Vec::new(),
                laps: Vec::new(),
                paused: true, // stopped by default; start by explicitly calling `.resume()`
            }
        }

        fn last_start(&self) -> DateTime<Local> {
            self.start_moments[self.start_moments.len() - 1]
        }
        fn last_lap(&self) -> DateTime<Local> {
            self.lap_moments[self.lap_moments.len() - 1]
        }
        fn pause(&mut self) {
            assert!(self.paused == false, "Already paused!");
            let moment = Local::now();
            self.pause_moments.push(moment);
            self.elapsed = self.elapsed + (moment - self.last_start());
            self.lap_elapsed = self.read_lap_elapsed(moment);
            self.paused = true;
        }
        fn resume(&mut self) {
            assert!(self.paused == true, "Already running!");
            self.start_moments.push(Local::now());
            self.paused = false;
        }
        fn pause_or_resume(&mut self) {
            if self.paused {
                self.resume();
            } else {
                self.pause();
            }
        }
        fn lap(&mut self) -> Option<Duration> {
            // assert!(!self.paused, "Paused!");
            if self.paused {
                None
            } else {
                let moment = Local::now();
                let lap = self.read_lap_elapsed(moment);
                self.lap_moments.push(moment);
                self.laps.push(lap);
                self.lap_elapsed = Duration::zero();
                Some(lap)
            }
        }
        /// Read the total time elapsed
        fn read(&self) -> Duration {
            if self.paused {
                self.elapsed
            } else {
                self.elapsed + (Local::now() - self.last_start())
            }
        }
        /// Read the time elapsed in the current lap
        fn read_lap_elapsed(&self, moment: DateTime<Local>) -> Duration {
            self.lap_elapsed
                + if self.lap_elapsed == Duration::zero()
                    && !self.lap_moments.is_empty()
                {
                    moment - self.last_lap()
                } else {
                    moment - self.last_start()
                }
        }
    }

    pub struct StopWatchView {
        stopwatch: StopWatch,
        on_stop: Option<Rc<dyn Fn(&mut Cursive, Duration)>>,
        show_laps: usize,
    }

    impl StopWatchView {
        pub fn new() -> Self {
            Self {
                stopwatch: StopWatch::new(),
                on_stop: None,
                show_laps: 0,
            }
        }

        pub fn with_laps(mut self, n: usize) -> Self {
            self.show_laps = n;
            self
        }

        /// Sets a callback to be used when `<Enter>` is pressed.
        ///
        /// The elapsed time will be given to the callback.
        ///
        /// See also cursive::views::select_view::SelectView::set_on_submit
        pub fn set_on_stop<F, R>(&mut self, cb: F)
        where
            F: 'static + Fn(&mut Cursive, Duration) -> R,
        {
            self.on_stop = Some(Rc::new(move |s, t| {
                cb(s, t);
            }));
        }

        pub fn on_stop<F, R>(self, cb: F) -> Self
        where
            F: 'static + Fn(&mut Cursive, Duration) -> R,
        {
            self.with(|s| s.set_on_stop(cb))
        }

        fn stop(&mut self) -> EventResult {
            let stopwatch = &mut self.stopwatch;
            if !stopwatch.paused {
                stopwatch.pause();
            }
            let result = if self.on_stop.is_some() {
                let cb = self.on_stop.clone().unwrap();
                let elapsed = stopwatch.elapsed;
                EventResult::with_cb(move |s| cb(s, elapsed))
            } else {
                EventResult::Consumed(None)
            };
            // reset the stopwatch data, but not other configurations related to the `View`
            self.stopwatch = StopWatch::new();
            // return result
            result
        }
    }
    impl View for StopWatchView {
        fn draw(&self, printer: &Printer) {
            printer.print((4, 0), &self.stopwatch.read().pretty());
            let len = self.stopwatch.laps.len();
            for i in 1..=std::cmp::min(len, self.show_laps) {
                printer.print(
                    (0, i),
                    &[
                        format!("Lap {:02}: ", len - i + 1),
                        self.stopwatch.laps[len - i].pretty(),
                    ]
                    .concat(),
                );
            }
        }

        fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
            // the required size depends on how many lap times the user want to diaplay
            Vec2::new(20, self.show_laps + 1) // columns, rows (width, height)
        }

        fn on_event(&mut self, event: Event) -> EventResult {
            match event {
                // pause/resume the stopwatch when pressing "Space"
                Event::Char(' ') => {
                    self.stopwatch.pause_or_resume();
                }
                Event::Key(Key::Enter) => {
                    return self.stop();
                }
                Event::Char('l') => {
                    self.stopwatch.lap();
                }
                _ => return EventResult::Ignored,
            }
            EventResult::Consumed(None)
        }
    }
}

pub trait PrettyDuration {
    fn pretty(&self) -> String;
}
impl PrettyDuration for chrono::Duration {
    /// Pretty-prints a chrono::Duration in the form `HH:MM:SS.xxx`
    fn pretty(&self) -> String {
        let s = self.num_seconds();
        let ms = self.num_milliseconds() - 1000 * s;
        let (h, s) = (s / 3600, s % 3600);
        let (m, s) = (s / 60, s % 60);
        format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
    }
}
