extern crate cursive;
extern crate rand;

use cursive::Cursive;
use cursive::traits::*;
use cursive::views::{Button, Dialog, LinearLayout, ProgressBar, TextView};
use cursive::views::Counter;
use rand::Rng;
use std::cmp::min;
use std::thread;
use std::time::Duration;

// This example shows a ProgressBar reporting the status from an asynchronous
// job.
//
// It works by sharing a counter with the job thread. This counter can be
// "ticked" to indicate progress.

fn main() {
    let mut siv = Cursive::new();

    // We'll start slowly with a simple start button...
    siv.add_layer(
        Dialog::new()
            .title("Progress bar example")
            .padding((0, 0, 1, 1))
            .content(Button::new("Start", phase_1)),
    );

    // Auto-refresh is currently required for animated views
    siv.set_fps(30);

    siv.run();
}

fn phase_1(s: &mut Cursive) {
    // Phase 1 is easy: a simple pre-loading.

    // Number of ticks
    let n_max = 500;

    // This is the callback channel
    let cb = s.cb_sink().clone();

    s.pop_layer();
    s.add_layer(Dialog::around(
        ProgressBar::new()
            // We need to know how many ticks represent a full bar.
            .range(0, n_max)
            .with_task(move |counter| {
                // This closure will be called in a separate thread.
                fake_load(n_max, &counter);

                // When we're done, send a callback through the channel
                cb.send(Box::new(coffee_break)).unwrap();
            })
            .full_width(),
    ));
}

fn coffee_break(s: &mut Cursive) {
    // A little break before things get serious.
    s.pop_layer();
    s.add_layer(
        Dialog::new()
            .title("Preparation complete")
            .content(TextView::new("Now, the real deal!").center())
            .button("Again??", phase_2),
    );
}

fn phase_2(s: &mut Cursive) {
    // Now, we'll run N tasks
    // (It could be downloading a file, extracting an archive,
    // reticulating sprites, ...)
    let n_bars = 10;
    // Each task will have its own shiny counter
    let counters: Vec<_> = (0..n_bars).map(|_| Counter::new(0)).collect();
    // To make things more interesting, we'll give a random speed to each bar
    let speeds: Vec<_> = (0..n_bars)
        .map(|_| rand::thread_rng().gen_range(50, 150))
        .collect();

    let n_max = 100_000;
    let cb = s.cb_sink().clone();

    // Let's prepare the progress bars...
    let mut linear = LinearLayout::vertical();
    for c in &counters {
        linear.add_child(ProgressBar::new().max(n_max).with_value(c.clone()));
    }

    s.pop_layer();
    s.add_layer(Dialog::around(linear.full_width()).title("Just a moment..."));

    // And we start the worker thread.
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(5));
            let mut done = true;
            for (c, s) in counters.iter().zip(&speeds) {
                let ticks = min(n_max - c.get(), *s);
                c.tick(ticks);
                if c.get() < n_max {
                    done = false;
                }
            }
            if done {
                break;
            }
        }

        cb.send(Box::new(final_step)).unwrap();
    });
}

fn final_step(s: &mut Cursive) {
    // A little break before things get serious.
    s.pop_layer();
    s.add_layer(
        Dialog::new()
            .title("Report")
            .content(
                TextView::new(
                    "Time travel was a success!\n\
                     We went forward a few seconds!!",
                ).center(),
            )
            .button("That's it?", |s| s.quit()),
    );
}

// Function to simulate a long process.
fn fake_load(n_max: usize, counter: &Counter) {
    for _ in 0..n_max {
        thread::sleep(Duration::from_millis(5));
        // The `counter.tick()` method increases the progress value
        counter.tick(1);
    }
}
