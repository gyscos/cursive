extern crate cursive;

use cursive::Cursive;
use cursive::view::{Dialog,SelectView,TextView,Selector};

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Dialog::new(SelectView::new()
                              .item_str("Berlin")
                              .item_str("London")
                              .item_str("New York")
                              .item_str("Paris")
                              .with_id("city"))
                  .title("Where are you from?")
                  .button("Ok", |s| {
                      let city = s.find::<SelectView>(&Selector::Id("city")).unwrap().selection().to_string();
                      s.pop_layer();
                      s.add_layer(Dialog::new(TextView::new(&format!("{} is a great city!", city)))
                          .button("Quit", |s| s.quit()));
                  }));

    siv.run();
}

