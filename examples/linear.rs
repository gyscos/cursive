extern crate cursive;

use cursive::Cursive;
use cursive::align::HAlign;
use cursive::traits::*;
use cursive::views::{Dialog, DummyView, LinearLayout, TextView};

// This example uses a LinearLayout to stick multiple views next to each other.

fn main() {
    let mut siv = Cursive::new();

    // Some description text
    let text = "This is a very simple example of linear layout. Two views \
                are present, a short title above, and this text. The text \
                has a fixed width, and the title is centered horizontally.";

    // We'll create a dialog with a TextView serving as a title
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
            .child(TextView::new("Title").h_align(HAlign::Center))
            // Dummy views can be used for spacing
            .child(DummyView.fixed_height(1))
            // Disabling scrolling means the view cannot shrink.
            // Try resizing the window, and see what happens!
            .child(TextView::new(text).scrollable(false))
            .child(TextView::new(text))
            .child(TextView::new(text))
            .child(TextView::new(text))
            // Give everything a fixed width so it doesn't get too wide
            .fixed_width(30),
        ).button("Quit", |s| s.quit())
            .h_align(HAlign::Center),
    );

    siv.run();
}
