extern crate cursive;

use std::fs::File;
use std::io::{BufReader,BufRead};

use cursive::Cursive;
use cursive::view::{Dialog,SelectView,TextView,Selector,BoxView};

fn main() {

    let mut select = SelectView::new();

    // Read the list of cities from separate file, and fill the view with it.
    let file = File::open("assets/cities.txt").unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        select.add_item_str(&line.unwrap());
    }

    let mut siv = Cursive::new();

    siv.add_layer(Dialog::new(BoxView::new((20,10), select.with_id("city")))
                  .title("Where are you from?")
                  .button("Ok", |s| {
                      let city = s.find::<SelectView>(&Selector::Id("city")).unwrap().selection().to_string();
                      s.pop_layer();
                      s.add_layer(Dialog::new(TextView::new(&format!("{} is a great city!", city)))
                          .button("Quit", |s| s.quit()));
                  }));

    siv.run();
}

