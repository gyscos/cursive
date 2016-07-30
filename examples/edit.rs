extern crate cursive;

use cursive::prelude::*;

fn main() {
    let mut siv = Cursive::new();

    // Create a dialog with an edit text and a button.
    siv.add_layer(Dialog::empty()
                  .title("Enter your name")
                  .padding((1, 1, 1, 0))
                  .content(EditView::new()
                           .min_length(20)
                           .on_submit(|s, name| {
                               if name.is_empty() {
                                   s.add_layer(Dialog::new(TextView::new("Please enter a name!"))
                                               .dismiss_button("Ok"));
                               } else {
                                   let content = format!("Hello {}!", name);
                                   s.pop_layer();
                                   s.add_layer(Dialog::new(TextView::new(&content))
                                               .button("Quit", |s| s.quit()));
                               }
                           }))
                 );

    siv.run();
}
