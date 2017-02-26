extern crate cursive;

use cursive::Cursive;
use cursive::view::{Boxable, Identifiable};
use cursive::views::{LinearLayout, EditView, TextView, Dialog};

fn main() {
    let mut siv = Cursive::new();

    // Create a dialog with 2 edit fields, and a text view.
    // The text view indicates when the 2 fields content match.
    siv.add_layer(Dialog::around(LinearLayout::vertical()
            .child(EditView::new().on_edit(on_edit).with_id_mut("1"))
            .child(EditView::new().on_edit(on_edit).with_id_mut("2"))
            .child(TextView::new("match").with_id_mut("match"))
            .fixed_width(10))
        .button("Quit", Cursive::quit));

    siv.run();
}

// Compare the content of the two edit views,
// and update the TextView accordingly.
//
// We'll ignore the `content` and `cursor` arguments,
// and directly retrieve the content from the `Cursive` root.
fn on_edit(siv: &mut Cursive, _content: &str, _cursor: usize) {
    // Get handles for each view.
    let edit_1 = siv.find_id_mut::<EditView>("1").unwrap();
    let edit_2 = siv.find_id_mut::<EditView>("2").unwrap();
    let mut text = siv.find_id_mut::<TextView>("match").unwrap();

    // Directly compare references to edit_1 and edit_2.
    text.set_content(if edit_1.get_content() == edit_2.get_content() {
        "match"
    } else {
        "no match"
    });
}
