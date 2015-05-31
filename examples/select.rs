extern crate cursive;

use std::fs::File;
use std::io::{BufReader,BufRead};

use cursive::Cursive;
use cursive::view::{Dialog,SelectView,TextView,Selector,BoxView};

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
    siv.add_layer(Dialog::new(BoxView::new((20,10), select.with_id("city")))
                  .title("Where are you from?")
                  .button("Ok", |s| {
                      // Find the SelectView, gets its selection.
                      let city = s.find::<SelectView>(&Selector::Id("city")).unwrap().selection().to_string();
                      // And show the next window.
                      s.pop_layer();
                      s.add_layer(Dialog::new(TextView::new(&format!("{} is a great city!", city)))
                          .button("Quit", |s| s.quit()));
                  }));

    siv.run();
}

