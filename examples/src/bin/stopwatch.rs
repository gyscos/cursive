use cursive::traits::*;

fn main() {
    let mut siv = cursive::default();
    let timer = Timer::TimerView::new();
    siv.add_layer(timer.fixed_width(8).with_name("timer"));
    siv.add_layer(cursive::views::Dialog::info(
        "Press 'Space' to start/pause/resume the timer!",
    ));
    siv.set_fps(5);
    siv.run();
}

mod Timer {
    use chrono::{DateTime, Duration, Local};
    use cursive::{
        event::{Callback, Event, EventResult, Key},
        view::View,
        Cursive, Printer, Vec2,
    };
    use std::rc::Rc;

    /// ```ignore
    ///                  lap    lap          lap
    /// start       start |      |     start  |
    ///   o--------x   o-----------x      o-----------x
    ///          pause           pause            pause(end)
    /// ```
    pub struct TimerView {
        pub elapsed: Duration,
        pub lap_elapsed: Duration,
        paused: bool,
        pub pause_moments: Vec<DateTime<Local>>,
        pub start_moments: Vec<DateTime<Local>>,
        pub lap_moments: Vec<DateTime<Local>>,
        pub laps: Vec<Duration>,
        on_stop: Option<Rc<dyn Fn(&mut Cursive, Duration)>>,
    }

    impl TimerView {
        pub fn new() -> Self {
            Self {
                elapsed: Duration::zero(),
                lap_elapsed: Duration::zero(),
                on_stop: None,
                start_moments: Vec::new(),
                pause_moments: Vec::new(),
                lap_moments: Vec::new(),
                laps: Vec::new(),
                paused: true,
            }
        }
        pub fn last_start(&self) -> DateTime<Local> {
            self.start_moments[self.start_moments.len() - 1]
        }
        pub fn last_pause(&self) -> DateTime<Local> {
            self.pause_moments[self.pause_moments.len() - 1]
        }
        pub fn last_lap(&self) -> DateTime<Local> {
            self.lap_moments[self.lap_moments.len() - 1]
        }
        pub fn pause(&mut self) {
            assert!(self.paused == false, "Already paused!");
            let moment = Local::now();
            self.pause_moments.push(moment);
            self.elapsed = self.elapsed + (moment - self.last_start());
            self.lap_elapsed = self.read_lap_elapsed(moment);
            self.paused = true;
        }
        pub fn resume(&mut self) {
            assert!(self.paused == true, "Already running!");
            self.start_moments.push(Local::now());
            self.paused = false;
        }
        pub fn pause_or_resume(&mut self) {
            if self.paused {
                self.resume();
            } else {
                self.pause();
            }
        }
        pub fn lap(&mut self) -> Option<Duration> {
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
        pub fn read(&self) -> Duration {
            if self.paused {
                self.elapsed
            } else {
                self.elapsed + (Local::now() - self.last_start())
            }
        }
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

        /// Sets a callback to be used when `<Enter>` is pressed.
        ///
        /// Also happens if the user clicks an item.
        ///
        /// The elapsed time will be given to the callback.
        ///
        /// See also cursive::views::select_view::SelectView::on_submit
        pub fn on_stop<F, R>(&mut self, cb: F)
        where
            F: 'static + Fn(&mut Cursive, Duration) -> R,
        {
            self.on_stop = Some(Rc::new(move |s, t| {
                cb(s, t);
            }));
            unimplemented!();
        }

        // fn stop(&mut self) -> EventResult {
        //     self.pause();
        //     let cb = self.on_stop.clone().unwrap();
        //     // We return a Callback Rc<|s| cb(s, &*v)>
        //     EventResult::Consumed(Some(Callback::from_fn(move |s| {
        //         cb(s, self.elapsed)
        //     })))
        // }
    }
    impl View for TimerView {
        fn draw(&self, printer: &Printer) {
            printer.print((0, 0), &pretty(self.read()));
            for i in 0..self.laps.len() {
                printer.print((0, i + 1), &pretty(self.laps[i]));
            }
        }

        fn required_size(&mut self, constraint: Vec2) -> Vec2 {
            let _ = constraint;
            Vec2::new(8, 8)
        }

        fn on_event(&mut self, event: Event) -> EventResult {
            match event {
                // pause/resume the timer when pressing "Space"
                Event::Char(' ') => {
                    self.pause_or_resume();
                }
                // Event::Key(Key::Enter) if self.on_stop.is_some() => {
                //     return self.stop();
                // }
                Event::Char('l') => {
                    self.lap();
                }
                _ => return EventResult::Ignored,
            }
            EventResult::Consumed(None)
        }
    }
    /// pretty-prints a `chrono::Duration` in the form "HH:MM:SS"
    fn pretty(duration: Duration) -> String {
        let s = duration.num_seconds();
        let (h, s) = (s / 3600, s % 3600);
        let (m, s) = (s / 60, s % 60);
        format!("{:02}:{:02}:{:02}", h, m, s)
    }
}
