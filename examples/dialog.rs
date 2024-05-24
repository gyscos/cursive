use cursive::{
    views::{CircularFocus, Dialog, TextView},
    With as _,
};

fn main() {
    // Creates the cursive root - required for every application.
    let mut siv = cursive::default();

    // Creates a dialog with a single "Quit" button
    siv.add_layer(
        // Most views can be configured in a chainable way
        Dialog::around(TextView::new("Hello Dialog!"))
            .title("Cursive")
            .button("Foo", |_s| ())
            .button("Quit", |s| s.quit())
            .wrap_with(CircularFocus::new)
            .wrap_tab(),
    );

    // Starts the event loop.
    siv.run();
}
