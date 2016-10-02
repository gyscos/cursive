extern crate cursive;

use cursive::Cursive;
use cursive::views::{Dialog, TextView};
use cursive::align::HAlign;

fn main() {
    // Read some long text from a file.
    let content = include_str!("../assets/lorem.txt");

    let mut siv = Cursive::new();

    // We can quit by pressing q
    siv.add_global_callback('q', |s| s.quit());

    // The text is too long to fit on a line, so the view will wrap lines,
    // and will adapt to the terminal size.
    siv.add_layer(Dialog::around(TextView::new(content))
        .h_align(HAlign::Center)
        .button("Quit", |s| s.quit()));
    // Show a popup on top of the view.
    siv.add_layer(Dialog::info("Try resizing the terminal!\n(Press 'q' to \
                                quit when you're done.)"));

    siv.run();
}
