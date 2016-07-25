extern crate cursive;

use cursive::prelude::*;

use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Dialog::new(Button::new("Start", |s| {
            // These two values will allow us to communicate.
            let value = Arc::new(AtomicUsize::new(0));
            let cb = Arc::new(Mutex::new(None));

            let n_max = 1000;

            s.find_id::<Dialog>("dialog")
                .unwrap()
                .set_content(ProgressBar::new()
                    .range(0, n_max)
                    .with_value(value.clone())
                    .with_callback(cb.clone()));

            // Spawn a thread to process things in the background.
            thread::spawn(move || {
                for _ in 0..n_max {
                    thread::sleep(Duration::from_millis(3));
                    value.fetch_add(1, Ordering::Relaxed);
                }
                *cb.lock().unwrap() = Some(Box::new(move |s| {
                    s.pop_layer();
                    s.add_layer(Dialog::new(TextView::new("Phew, that was \
                                                           a lot of work!"))
                        .title("Work done!")
                        .button("Sure!", |s| s.quit()));
                }));
            });

        }))
        .title("Progress bar example")
        .padding((0,0,1,1))
        .with_id("dialog"));

    siv.set_fps(10);

    siv.run();
}
