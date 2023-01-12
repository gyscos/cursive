//! A simple stopwatch implementation.
//! Also check out [`clock-cli`](https://github.com/TianyiShi2001/cl!ock-cli-rs),
//! which aims to implement a fully-fledged clock with stopwatch, countdown
//! timer, and possibly more functionalities.

use cursive::traits::{Nameable, Resizable};
use cursive::views::{Button, Canvas, Dialog, LinearLayout};
use std::time::{Duration, Instant};

fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        Dialog::new()
            .title("Stopwatch")
            .content(
                LinearLayout::horizontal()
                    .child(
                        Canvas::new(Watch {
                            last_started: Instant::now(),
                            last_elapsed: Duration::default(),
                            running: true,
                        })
                        .with_draw(|s, printer| {
                            printer.print((0, 1), &format!("{:.2?}", s.elapsed()));
                        })
                        .with_name("stopwatch")
                        .fixed_size((8, 3)),
                    )
                    .child(
                        LinearLayout::vertical()
                            .child(Button::new("Start", run(Watch::start)))
                            .child(Button::new("Pause", run(Watch::pause)))
                            .child(Button::new("Stop", run(Watch::stop))),
                    ),
            )
            .button("Quit", |s| s.quit())
            .h_align(cursive::align::HAlign::Center),
    );

    siv.set_fps(20);

    siv.run();
}

struct Watch {
    last_started: Instant,
    last_elapsed: Duration,
    running: bool,
}

impl Watch {
    fn start(&mut self) {
        if self.running {
            return;
        }
        self.running = true;
        self.last_started = Instant::now();
    }

    fn elapsed(&self) -> Duration {
        self.last_elapsed
            + if self.running {
                Instant::now() - self.last_started
            } else {
                Duration::default()
            }
    }

    fn pause(&mut self) {
        self.last_elapsed = self.elapsed();
        self.running = false;
    }

    fn stop(&mut self) {
        self.running = false;
        self.last_elapsed = Duration::default();
    }
}

// Helper function to find the stopwatch view and run a closure on it.
fn run<F>(f: F) -> impl Fn(&mut cursive::Cursive)
where
    F: Fn(&mut Watch),
{
    move |s| {
        s.call_on_name("stopwatch", |c: &mut Canvas<Watch>| {
            f(c.state_mut());
        });
    }
}
