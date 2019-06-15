use cursive::views::{CircularFocus, Dialog, TextView};
use cursive::Cursive;

fn main() {
    // Creates the cursive root - required for every application.
    let mut siv = Cursive::default();

    // Creates a dialog with a single "Quit" button
    siv.add_layer(
        // Most views can be configured in a chainable way
        CircularFocus::wrap_tab(
            Dialog::around(TextView::new("Hello Dialog!"))
                .title("Cursive")
                .button("Foo", |_s| ())
                .button("Quit", |s| s.quit()),
        ),
    );

    // Starts the event loop.
    siv.run();
}
