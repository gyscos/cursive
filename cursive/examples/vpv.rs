use std::io;

use cursive::traits::{Resizable, With};
use cursive::utils;
use cursive::views::{Canvas, Dialog, LinearLayout, ProgressBar};
use cursive::Cursive;
use pretty_bytes::converter::convert;
use std::thread;
use std::time;

// This example is a visual version of the `pv` tool.

fn main() {
    let mut siv = cursive::default();

    // We'll use this channel to signal the end of the transfer
    let cb_sink = siv.cb_sink().clone();

    // Use a counter to track progress
    let counter = utils::Counter::new(0);
    let counter_copy = counter.clone();
    let start = time::Instant::now();

    // If an argument is given, it is the file we'll read from.
    // Otherwise, read from stdin.
    let (source, len) = match std::env::args().nth(1) {
        Some(source) => {
            let meta = std::fs::metadata(&source).unwrap();
            // If possible, read the file size to have a progress bar.
            let len = meta.len();
            (Some(source), if len > 0 { Some(len) } else { None })
        }
        None => (None, None),
    };

    // Add a single view: progress status
    siv.add_layer(
        Dialog::new()
            .title("Copying...")
            .content(
                LinearLayout::vertical()
                    .child(
                        Canvas::new(counter.clone())
                            .with_draw(move |c, printer| {
                                let ticks = c.get() as f64;
                                let now = time::Instant::now();
                                let duration = now - start;

                                let seconds = duration.as_secs() as f64
                                    + f64::from(duration.subsec_nanos())
                                        * 1e-9;

                                let speed = ticks / seconds;

                                // Print ETA if we have a file size
                                // Otherwise prints elapsed time.
                                if let Some(len) = len {
                                    let remaining =
                                        (len as f64 - ticks) / speed;
                                    printer.print(
                                        (0, 0),
                                        &format!(
                                            "ETA:     {:.1} seconds",
                                            remaining
                                        ),
                                    );
                                } else {
                                    printer.print(
                                        (0, 0),
                                        &format!(
                                            "Elapsed: {:.1} seconds",
                                            seconds
                                        ),
                                    );
                                }
                                printer.print(
                                    (0, 1),
                                    &format!("Copied:  {}", convert(ticks)),
                                );
                                printer.print(
                                    (0, 2),
                                    &format!("Speed:   {}/s", convert(speed)),
                                );
                            })
                            .fixed_size((25, 3)),
                    )
                    .with(|l| {
                        // If we have a file length, add a progress bar
                        if let Some(len) = len {
                            l.add_child(
                                ProgressBar::new()
                                    .max(len as usize)
                                    .with_value(counter.clone()),
                            );
                        }
                    }),
            )
            .button("Abort", Cursive::quit),
    );

    if source.is_none() && atty::is(atty::Stream::Stdin) {
        siv.add_layer(
            Dialog::text(
                "Please specify an input file or redirect a file to stdin.

cargo run --example vpv </dev/zero >/dev/null",
            )
            .button("Quit", Cursive::quit),
        );
    } else {
        // Start the copy in a separate thread
        thread::spawn(move || {
            // Copy to stdout - lock it for better performance.
            let stdout = io::stdout();
            let mut stdout = stdout.lock();

            match source {
                None => {
                    // Copy from stdin - lock it for better performance.
                    let stdin = io::stdin();
                    let stdin = stdin.lock();
                    let mut reader =
                        utils::ProgressReader::new(counter_copy, stdin);

                    // And copy!
                    io::copy(&mut reader, &mut stdout).unwrap();
                }
                Some(source) => {
                    // Copy from stdin - lock it for better performance.
                    let input = std::fs::File::open(source).unwrap();
                    let mut reader =
                        utils::ProgressReader::new(counter_copy, input);

                    // And copy!
                    io::copy(&mut reader, &mut stdout).unwrap();
                }
            }

            // When we're done, shut down the application
            cb_sink.send(Box::new(Cursive::quit)).unwrap();
        });
        siv.set_autorefresh(true);
    }

    siv.run();
}
