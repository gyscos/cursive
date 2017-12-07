extern crate cursive;

use cursive::Cursive;
use cursive::event::{Event, Key};
use cursive::traits::*;
use cursive::views::{Dialog, EditView, OnEventView, TextArea};

fn main() {
    let mut siv = Cursive::new();

    // The main dialog will just have a textarea.
    // Its size expand automatically with the content.
    siv.add_layer(
        Dialog::new()
            .title("Describe your issue")
            .padding((1, 1, 1, 0))
            .content(TextArea::new().with_id("text"))
            .button("Ok", Cursive::quit),
    );

    // We'll add a find feature!
    siv.add_layer(Dialog::info("Hint: press Ctrl-F to find in text!"));

    siv.add_global_callback(Event::CtrlChar('f'), |s| {
        // When Ctrl-F is pressed, show the Find popup.
        // Pressing the Escape key will discard it.
        s.add_layer(
            OnEventView::new(
                Dialog::new()
                    .title("Find")
                    .content(
                        EditView::new()
                            .on_submit(find)
                            .with_id("edit")
                            .min_width(10),
                    )
                    .button("Ok", |s| {
                        let text = s.call_on_id(
                            "edit",
                            |view: &mut EditView| view.get_content(),
                        ).unwrap();
                        find(s, &text);
                    })
                    .button("Cancel", |s| s.pop_layer()),
            ).on_event(Event::Key(Key::Esc), |s| s.pop_layer()),
        )
    });

    siv.run();
}

fn find(siv: &mut Cursive, text: &str) {
    // First, remove the find popup
    siv.pop_layer();

    let res = siv.call_on_id("text", |v: &mut TextArea| {
        // Find the given text from the text area content
        // Possible improvement: search after the current cursor.
        if let Some(i) = v.get_content().find(text) {
            // If we found it, move the cursor
            v.set_cursor(i);
            Ok(())
        } else {
            // Otherwise, return an error so we can show a warning.
            Err(())
        }
    });

    if let Some(Err(())) = res {
        // If we didn't find anything, tell the user!
        siv.add_layer(Dialog::info(format!("`{}` not found", text)));
    }
}
