extern crate cursive;

use cursive::Cursive;
use cursive::views::{Dialog, LinearLayout, TextView};
use cursive::align::HAlign;
use cursive::traits::*;

fn main() {
    let mut siv = Cursive::new();

    // Some description text
    let text = "This is a very simple example of linear layout. Two views \
                are present, a short title above, and this text. The text \
                has a fixed width, and the title is centered horizontally.";

    // We'll create a dialog with a TextView serving as a title
    siv.add_layer(Dialog::around(LinearLayout::vertical()
            .child(TextView::new("Title").h_align(HAlign::Center))
            // Box the textview, so it doesn't get too wide.
            // A 0 height value means it will be unconstrained.
            .child(TextView::new(text).scrollable(false).fixed_width(30))
            .child(TextView::new(text).fixed_width(30))
            .child(TextView::new(text).fixed_width(30))
            .child(TextView::new(text).fixed_width(30)))
        .button("Quit", |s| s.quit())
        .h_align(HAlign::Center));

    siv.run();
}
