extern crate cursive;

use cursive::Cursive;
use cursive::views::{Dialog, SelectView, TextView};
use cursive::align::HAlign;
use cursive::traits::*;

fn main() {
    let mut select = SelectView::new().h_align(HAlign::Center);

    // Read the list of cities from separate file, and fill the view with it.
    // (We include the file at compile-time to avoid runtime read errors.)
    let content = include_str!("../assets/cities.txt");
    select.add_all_str(content.lines());

    // Sets the callback for when "Enter" is pressed.
    select.set_on_submit(show_next_window);

    let mut siv = Cursive::new();

    // Let's add a BoxView to keep the list at a reasonable size - it can scroll anyway.
    siv.add_layer(Dialog::around(select.fixed_size((20, 10)))
        .title("Where are you from?"));

    siv.run();
}

// Let's put the callback in a separate function to keep it clean, but it's not required.
fn show_next_window(siv: &mut Cursive, city: &str) {
    siv.pop_layer();
    let text = format!("{} is a great city!", city);
    siv.add_layer(Dialog::around(TextView::new(text))
        .button("Quit", |s| s.quit()));
}
