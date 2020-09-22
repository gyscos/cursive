use cursive::traits::*;

fn main() {
    let mut siv = cursive::default();
    let timer = Timer::TimerView::new();
    siv.add_layer(timer.fixed_width(9).with_name("timer"));
    siv.add_layer(cursive::views::Dialog::info(
        "Press 'Space' to start/pause/resume the timer!",
    ));
    siv.set_fps(5);
    siv.run();
}

mod Timer {
    use chrono::{DateTime, Duration, Local};
    use cursive::{
        event::{Event, EventResult},
        view::View,
        Printer,
    };

    pub struct TimerView {
        elapsed: Duration,
        last_update: DateTime<Local>,
        paused: bool,
    }

    impl TimerView {
        pub fn new() -> Self {
            Self {
                elapsed: Duration::zero(),
                last_update: Local::now(),
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
                _ => return EventResult::Ignored,
            }
            EventResult::Consumed(None)
        }
    }
    /// pretty-prints a `chrono::Duration`
    fn pretty(duration: Duration) -> String {
        let s = duration.num_seconds();
        let (h, s) = (s / 3600, s % 3600);
        let (m, s) = (s / 60, s % 60);
        format!("{:02}:{:02}:{:02}", h, m, s)
    }
}
