extern crate cursive;

use cursive::prelude::*;

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Dialog::new(ListView::new()
            .child("Name", EditView::new().min_length(10))
            .child("Email",
                   LinearLayout::horizontal()
                       .child(EditView::new()
                           .min_length(15)
                           .disabled()
                           .with_id("email1"))
                       .child(TextView::new("@"))
                       .child(EditView::new()
                           .min_length(10)
                           .disabled()
                           .with_id("email2")))
            .child("Receive spam?",
                   Checkbox::new().on_change(|s, checked| {
                       for name in &["email1", "email2"] {
                           let view: &mut EditView = s.find_id(name).unwrap();
                           view.set_enabled(checked);
                       }
                   }))
            .delimiter()
            .child("Age",
                   SelectView::new().popup()
                       .item_str("0-18")
                       .item_str("19-30")
                       .item_str("31-40")
                       .item_str("41+"))
            .with(|list| {
                for i in 0..50 {
                    list.add_child(&format!("Item {}", i), EditView::new());
                }
            }))
        .title("Please fill out this form")
        .button("Ok", |s| s.quit()));

    siv.run();
}
