use cursive::traits::*;
use cursive::view::{Offset, Position};
use cursive::views::{Dialog, OnEventView, TextView};
use cursive::Cursive;

// This example modifies a view after creation.

fn main() {
    let mut siv = cursive::default();

    let content = "Press Q to quit the application.\n\nPress P to open the \
                   popup.";

    siv.add_global_callback('q', |s| s.quit());

    // Let's wrap the view to give it a recognizable name, so we can look for it.
    // We add the P callback on the textview only (and not globally),
    // so that we can't call it when the popup is already visible.
    siv.add_layer(
        OnEventView::new(TextView::new(content).with_name("text")).on_event('p', show_popup),
    );

    siv.run();
}

fn show_popup(siv: &mut Cursive) {
    // Let's center the popup horizontally, but offset it down a few rows,
    // so the user can see both the popup and the view underneath.
    siv.screen_mut().add_layer_at(
        Position::new(Offset::Center, Offset::Parent(5)),
        Dialog::around(TextView::new("Tak!"))
            .button("Change", |s| {
                // Look for a view tagged "text".
                // We _know_ it's there, so unwrap it.
                s.call_on_name("text", |view: &mut TextView| {
                    let content = reverse(view.get_content().source());
                    view.set_content(content);
                });
            })
            .dismiss_button("Ok"),
    );
}

// This just reverses each character
//
// Note: it would be more correct to iterate on graphemes instead.
// Check the unicode_segmentation crate!
fn reverse(text: &str) -> String {
    text.chars().rev().collect()
}
