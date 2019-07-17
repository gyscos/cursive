use cursive::align::HAlign;
use cursive::traits::*;
use cursive::views::{Dialog, DummyView, LinearLayout, TextView};
use cursive::Cursive;

// This example uses a LinearLayout to stick multiple views next to each other.

fn main() {
    let mut siv = Cursive::default();

    // Some description text. We want it to be long, but not _too_ long.
    let text = "This is a very simple example of linear layout. Two views \
                are present, a short title above, and this text. The text \
                has a fixed width, and the title is centered horizontally.";

    // We'll create a dialog with a TextView serving as a title
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(TextView::new("Title").h_align(HAlign::Center))
                // Use a DummyView as spacer
                .child(DummyView.fixed_height(1))
                // Disabling scrollable means the view cannot shrink.
                .child(TextView::new(text))
                // The other views will share the remaining space.
                .child(TextView::new(text).scrollable())
                .child(TextView::new(text).scrollable())
                .child(TextView::new(text).scrollable())
                .fixed_width(30),
        )
        .button("Quit", |s| s.quit())
        .h_align(HAlign::Center),
    );

    siv.run();
}
