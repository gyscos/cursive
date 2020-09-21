use chrono::Local;
use cursive::{traits::*, views::TextView, Cursive};
use std::{thread, time::Duration};

fn main() {
    let mut siv = cursive::default();
    siv.add_layer(TextView::new("").with_name("elapsed"));
    start(&mut siv);
    siv.run();
}

fn start(s: &mut Cursive) {
    let cb_sink = s.cb_sink().clone();
    let start_time = Local::now();
    thread::spawn(move || loop {
        let now = Local::now();
        let elapsed = now.signed_duration_since(start_time);
        cb_sink
            .send(Box::new(move |s| {
                s.call_on_name("elapsed", |view: &mut TextView| {
                    view.set_content(pretty(elapsed));
                });
            }))
            .unwrap();
        thread::sleep(Duration::from_secs(1));
    });
}

/// pretty-prints a `chrono::Duration`
fn pretty(duration: chrono::Duration) -> String {
    let s = duration.num_seconds();
    let (h, s) = (s / 3600, s % 3600);
    let (m, s) = (s / 60, s % 60);
    format!("{:02}:{:02}:{:02}", h, m, s)
}
