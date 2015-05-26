extern crate cursive;

use cursive::{Cursive};
use cursive::view::{Dialog,IdView,EditView,Selector,TextView};

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Dialog::new(IdView::new("edit", EditView::new()))
                  .title("Enter your name")
                  .button("Ok", |s| {
                      let content = {
                          let name = s.find::<EditView>(&Selector::Id("edit")).unwrap().get_content();
                          format!("Hello {}", name)
                      };
                      s.pop_layer();
                      s.add_layer(Dialog::new(TextView::new(&content))
                                  .button("Quit", |s| s.quit()));
                  }));

    siv.run();
}
