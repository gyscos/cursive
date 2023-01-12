use cursive::{
    traits::*,
    views::{Checkbox, Dialog, EditView, LinearLayout, ListView, SelectView, TextArea, TextView},
};

// This example uses a ListView.
//
// ListView can be used to build forms, with a list of inputs.

fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        Dialog::new()
            .title("Please fill out this form")
            .button("Ok", |s| s.quit())
            .content(
                ListView::new()
                    // Each child is a single-line view with a label
                    .child("Name", EditView::new().fixed_width(10))
                    .child("Presentation", TextArea::new().min_height(4))
                    .child(
                        "Receive spam?",
                        Checkbox::new().on_change(|s, checked| {
                            // Enable/Disable the next field depending on this checkbox
                            for name in &["email1", "email2"] {
                                s.call_on_name(name, |view: &mut EditView| {
                                    view.set_enabled(checked)
                                });
                                if checked {
                                    s.focus_name("email1").unwrap();
                                }
                            }
                        }),
                    )
                    .child(
                        "Email",
                        // Each child must have a height of 1 line,
                        // but we can still combine multiple views!
                        LinearLayout::horizontal()
                            .child(
                                EditView::new()
                                    .disabled()
                                    .with_name("email1")
                                    .fixed_width(15),
                            )
                            .child(TextView::new("@"))
                            .child(
                                EditView::new()
                                    .disabled()
                                    .with_name("email2")
                                    .fixed_width(10),
                            ),
                    )
                    // Delimiter currently are just a blank line
                    .delimiter()
                    .child(
                        "Age",
                        // Popup-mode SelectView are small enough to fit here
                        SelectView::new()
                            .popup()
                            .item_str("0-18")
                            .item_str("19-30")
                            .item_str("31-40")
                            .item_str("41+"),
                    )
                    .with(|list| {
                        // We can also add children procedurally
                        for i in 0..50 {
                            list.add_child(
                                &format!("Item {i}"),
                                EditView::new(),
                            );
                        }
                    })
                    .scrollable(),
            ),
    );

    siv.run();
}
