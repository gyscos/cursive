extern crate cursive;

use cursive::prelude::*;

use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Dialog::empty()
        .title("Progress bar example")
        .padding((0, 0, 1, 1))
        .content(Button::new("Start", |s| {
            // These two values will allow us to communicate.
            let value = Arc::new(AtomicUsize::new(0));

            let n_max = 1000;

            s.pop_layer();
            s.add_layer(Panel::new(FullView::full_width(ProgressBar::new()
                    .range(0, n_max)
                    .with_value(value.clone()))));

            let cb = s.cb_sink().clone();

            // Spawn a thread to process things in the background.
            thread::spawn(move || {
                for _ in 0..n_max {
                    thread::sleep(Duration::from_millis(20));
                    value.fetch_add(1, Ordering::Relaxed);
                }
                cb.send(Box::new(move |s| {
                        s.pop_layer();
                        s.add_layer(Dialog::empty()
                            .title("Work done!")
                            .content(TextView::new("Phew, that was some \
                                                    work!"))
                            .button("Sure!", |s| s.quit()));
                    }))
                    .unwrap();
            });

        }))
        .with_id("dialog"));

    siv.set_fps(30);

    siv.run();
}
