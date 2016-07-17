extern crate cursive;

use cursive::Cursive;
use cursive::view::{BoxView, Dialog, LinearLayout, TextView};
use cursive::align::HAlign;

fn main() {
    let mut siv = Cursive::new();

    // Some description text
    let text = "This is a very simple example of linear layout. Two views \
                are present, a short title above, and this text. The text \
                has a fixed width, and the title is centered horizontally.";

    // We'll create a dialog with a TextView serving as a title
    siv.add_layer(Dialog::new(LinearLayout::vertical()
            .child(TextView::new("Title").h_align(HAlign::Center))
            // Box the textview, so it doesn't get too wide.
            // A 0 height value means it will be unconstrained.
            .child(BoxView::fixed_width(30, TextView::new(text).scrollable(false)))
            .child(BoxView::fixed_width(30, TextView::new(text)))
            .child(BoxView::fixed_width(30, TextView::new(text)))
            .child(BoxView::fixed_width(30, TextView::new(text))))
        .button("Quit", |s| s.quit())
        .h_align(HAlign::Center));

    siv.run();
}
