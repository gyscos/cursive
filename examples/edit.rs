extern crate cursive;

use cursive::{Cursive};
use cursive::view::{Dialog,IdView,EditView,Selector,TextView};

fn main() {
    let mut siv = Cursive::new();

    // Create a dialog with an edit text and a button.
    siv.add_layer(Dialog::new(IdView::new("edit", EditView::new().min_length(20)))
                  .padding((1,1,1,0))
                  .title("Enter your name")
                  .button("Ok", |s| {
                      // When the button is clicked, read the text and print it in a new dialog.
                      let content = {
                          let name = s.find::<EditView>(&Selector::Id("edit")).unwrap().get_content();
                          format!("Hello {}!", name)
                      };
                      s.pop_layer();
                      s.add_layer(Dialog::new(TextView::new(&content))
                                  .button("Quit", |s| s.quit()));
                  }));

    siv.run();
}
