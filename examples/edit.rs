extern crate cursive;

use cursive::Cursive;
use cursive::traits::*;
use cursive::views::{Dialog, EditView, TextView};

fn main() {
    let mut siv = Cursive::new();

    // Create a dialog with an edit text and a button.
    // The user can either hit the <Ok> button,
    // or press Enter on the edit text.
    siv.add_layer(
        Dialog::new()
            .title("Enter your name")
            // Padding is (left, right, top, bottom)
            .padding((1, 1, 1, 0))
            .content(
                EditView::new()
                    .on_submit(show_popup)
                    .with_id("name")
                    .fixed_width(20),
            )
            .button("Ok", |s| {
                // This will run the given closure, *ONLY* if a view with the
                // correct type and the given ID is found.
                let name = s.call_on_id("name", |view: &mut EditView| {
                    // We can return content from the closure!
                    view.get_content()
                }).unwrap();

                // Run the next step
                show_popup(s, &name);
            }),
    );

    siv.run();
}

// This will replace the current layer with a new popup.
// If the name is empty, we'll show an error message instead.
fn show_popup(s: &mut Cursive, name: &str) {
    if name.is_empty() {
        s.add_layer(Dialog::info("Please enter a name!"));
    } else {
        let content = format!("Hello {}!", name);
        s.pop_layer();
        s.add_layer(
            Dialog::around(TextView::new(content))
                .button("Quit", |s| s.quit()),
        );
    }
}
