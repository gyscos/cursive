extern crate cursive;

use cursive::{Cursive};
use cursive::view::{Dialog,EditView,TextView};

fn main() {
    let mut siv = Cursive::new();

    // Create a dialog with an edit text and a button.
    siv.add_layer(Dialog::new(EditView::new().min_length(20).with_id("edit"))
                  .padding((1,1,1,0))
                  .title("Enter your name")
                  .button("Ok", |s| {
                      // When the button is clicked, read the text and print it in a new dialog.
                      let name = s.find_id::<EditView>("edit").unwrap().get_content().to_string();
                      if name.is_empty() {
                          s.add_layer(Dialog::new(TextView::new("Please enter a name!"))
                                      .dismiss_button("Ok"));
                      } else {
                          let content = format!("Hello {}!", name);
                          s.pop_layer();
                          s.add_layer(Dialog::new(TextView::new(&content))
                                      .button("Quit", |s| s.quit()));
                      }
                  }));

    siv.run();
}
