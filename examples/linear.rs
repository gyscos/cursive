extern crate cursive;

use cursive::Cursive;
use cursive::view::{Dialog,TextView,LinearLayout,BoxView};
use cursive::align::HAlign;
use cursive::orientation::Orientation;

fn main() {
    let mut siv = Cursive::new();

    // Some longish content
    let text = "This is a very simple example of linear layout. Two views are present, a short title above, and this text. The text has a fixed width, and the title is centered horizontally.";

    println!("Blaaah");

    siv.add_layer(
        Dialog::new(
            LinearLayout::new(Orientation::Vertical)
            .child(TextView::new("Title").h_align(HAlign::Center))
            .child(BoxView::new((30,0), TextView::new(text))))
        .button("Quit", |s| s.quit())
        .h_align(HAlign::Center));

    siv.run();
}
