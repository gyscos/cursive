use cursive::traits::*;
use cursive::views::{Dialog, EditView, TextView};
use cursive::Cursive;

fn main() {
    let mut siv = Cursive::default();

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
                    // Call `show_popup` when the user presses `Enter`
                    .on_submit(show_popup)
                    // Give the `EditView` a name so we can refer to it later.
                    .with_id("name")
                    // Wrap this in a `BoxView` with a fixed width.
                    // Do this _after_ `with_id` or the name will point to the
                    // `BoxView` instead of `EditView`!
                    .fixed_width(20),
            )
            .button("Ok", |s| {
                // This will run the given closure, *ONLY* if a view with the
                // correct type and the given ID is found.
                let name = s
                    .call_on_id("name", |view: &mut EditView| {
                        // We can return content from the closure!
                        view.get_content()
                    })
                    .unwrap();

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
        // Try again as many times as we need!
        s.add_layer(Dialog::info("Please enter a name!"));
    } else {
        let content = format!("Hello {}!", name);
        // Remove the initial popup
        s.pop_layer();
        // And put a new one instead
        s.add_layer(
            Dialog::around(TextView::new(content))
                .button("Quit", |s| s.quit()),
        );
    }
}
