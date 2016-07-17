extern crate cursive;

use cursive::Cursive;
use cursive::With;
use cursive::view::{ListView, Checkbox, Dialog, EditView, TextView, LinearLayout};

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Dialog::new(ListView::new()
                              .child("Name", EditView::new().min_length(10))
                              .child("Email", LinearLayout::horizontal()
                                    .child(EditView::new().min_length(15))
                                    .child(TextView::new("@"))
                                    .child(EditView::new().min_length(10)))
                              .child("Receive spam?", Checkbox::new())
                              .delimiter()
                              .with(|list| {
                                  for i in 0..50 {
                                    list.add_child(&format!("Item {}", i), EditView::new());
                                  }
                              })
                              )
                    .title("Please fill out this form")
                    .button("Ok", |s| s.quit()));

    siv.run();
}
