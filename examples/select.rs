extern crate cursive;

use std::fs::File;
use std::io::{BufReader,BufRead};

use cursive::Cursive;
use cursive::view::{Dialog,SelectView,TextView,BoxView};

fn main() {
    // To keep things simple, little error management is done here.
    // If you have an error, be sure to run this from the crate root, not from a sub directory.

    let mut select = SelectView::new();

    // Read the list of cities from separate file, and fill the view with it.
    let file = File::open("assets/cities.txt").unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        select.add_item_str(&line.unwrap());
    }

    let mut siv = Cursive::new();

    // Let's add a BoxView to keep the list at a reasonable size - it can scroll anyway.
    siv.add_layer(Dialog::new(BoxView::new((20,10), select.on_select(|s,city| show_next_window(s,city))))
                  .title("Where are you from?"));

    siv.run();
}

// Let's put the callback in a separate function to keep it clean, but it's not required.
fn show_next_window(siv: &mut Cursive, city: &str) {
    siv.pop_layer();
    siv.add_layer(Dialog::new(TextView::new(&format!("{} is a great city!", city)))
                .button("Quit", |s| s.quit()));
}
