extern crate cursive;

use cursive::prelude::*;

use std::thread;
use std::time::Duration;

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Dialog::empty()
        .title("Progress bar example")
        .padding((0, 0, 1, 1))
        .content(Button::new("Start", |s| {
            // Number of ticks
            let n_max = 1000;

            // This is the callback channel
            let cb = s.cb_sink().clone();

            s.pop_layer();
            s.add_layer(Panel::new(FullView::full_width(
                ProgressBar::new()
                    .range(0, n_max)
                    .with_task(move |ticker| {
                        // This closure will be called in a separate thread.
                        for _ in 0..n_max {
                            thread::sleep(Duration::from_millis(5));
                            // The ticker method increases the progress value
                            ticker(1);
                        }

                        // When we're done, send a callback through the channel
                        cb.send(Box::new(move |s| {
                            s.pop_layer();
                            s.add_layer(Dialog::empty()
                                        .title("Work done!")
                                        .content(TextView::new("Phew!"))
                                        .button("Finally!", |s| s.quit()));
                        }))
                        .unwrap();
                    })
            )));
        })));

    siv.set_fps(30);

    siv.run();
}
