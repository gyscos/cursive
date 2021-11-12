use cursive::theme::*;
use cursive::views::*;
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
        LinearLayout::vertical()
            .child(
                LinearLayout::horizontal()
                    .child(
                        Dialog::around(TextView::new("Select border theme"))
                            .title("Cursive")
                            .focusable(true)
                            .focus_callback(|s| {
                                s.update_theme(|t| match t.borders {
                                    BorderStyle::Simple => {
                                        t.borders = BorderStyle::Outset
                                    }
                                    BorderStyle::Outset => {
                                        t.borders = BorderStyle::None
                                    }
                                    BorderStyle::None => {
                                        t.borders = BorderStyle::Simple
                                    }
                                })
                            })
                            .button("Simple", |s| {
                                s.update_theme(|t| {
                                    t.borders = BorderStyle::Simple
                                })
                            })
                            .button("Outset", |s| {
                                s.update_theme(|t| {
                                    t.borders = BorderStyle::Outset
                                })
                            })
                            .button("None", |s| {
                                s.update_theme(|t| {
                                    t.borders = BorderStyle::None
                                })
                            })
                            .wrap_with(CircularFocus::new)
                            .wrap_tab(),
                    )
                    .child(
                        Dialog::around(EditView::new().on_submit(|_, _| ()))
                            .title("Cursive")
                            .focusable(true)
                            .button("Foo", |_s| ())
                            .button("Quit", |s| s.quit())
                            .wrap_with(CircularFocus::new)
                            .wrap_tab(),
                    ),
            )
            .child(
                Dialog::around(TextView::new("Hello Dialog!"))
                    .title("Cursive")
                    .focusable(true)
                    .button("Foo", |_s| ())
                    .button("Quit", |s| s.quit())
                    .wrap_with(CircularFocus::new)
                    .wrap_tab(),
            ),
    );

    // Starts the event loop.
    siv.run();
}
