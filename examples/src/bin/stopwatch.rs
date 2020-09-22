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
        Cursive, Printer,
    };
    use std::rc::Rc;

    pub struct TimerView {
        elapsed: Duration,
        last_update: DateTime<Local>,
        paused: bool,
        on_stop: Option<Rc<dyn Fn(&mut Cursive, Duration)>>,
    }

    impl TimerView {
        pub fn new() -> Self {
            Self {
                elapsed: Duration::zero(),
                last_update: Local::now(),
                on_stop: None,
                paused: true,
            }
        }
        pub fn pause(&mut self) {
            assert!(self.paused == false, "Already paused!");
            self.elapsed = self.read();
            self.paused = true;
        }
        pub fn resume(&mut self) {
            assert!(self.paused == true, "Already running!");
            self.last_update = Local::now();
            self.paused = false;
        }
        pub fn pause_or_resume(&mut self) {
            if self.paused {
                self.resume();
            } else {
                self.pause();
            }
        }
        pub fn read(&self) -> Duration {
            if self.paused {
                self.elapsed
            } else {
                self.elapsed + (Local::now() - self.last_update)
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
        }

        fn stop(&mut self) -> EventResult {
            self.pause();
            let cb = self.on_stop.clone().unwrap();
            // We return a Callback Rc<|s| cb(s, &*v)>
            EventResult::Consumed(Some(Callback::from_fn(move |s| {
                cb(s, self.elapsed)
            })))
        }
    }
    impl View for TimerView {
        fn draw(&self, printer: &Printer) {
            printer.print((0, 0), &pretty(self.read()));
        }

        fn on_event(&mut self, event: Event) -> EventResult {
            match event {
                // pause/resume the timer when pressing "Space"
                Event::Char(' ') => {
                    self.pause_or_resume();
                }
                Event::Key(Key::Enter) if self.on_stop.is_some() => {
                    return self.stop();
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
