extern crate cursive;

use cursive::Cursive;
use cursive::views::{Dialog, EditView, TextView};
use cursive::traits::*;

fn main() {
    let mut siv = Cursive::new();

    // Create a dialog with an edit text and a button.
    // The user can either hit the <Ok> button,
    // or press Enter on the edit text.
    siv.add_layer(Dialog::new()
        .title("Enter your name")
        .padding((1, 1, 1, 0))
        .content(EditView::new()
            .on_submit(show_popup)
            .with_id("name")
            .fixed_width(20))
        .button("Ok", |s| {
            let name = s.find_id::<EditView>("name")
                .unwrap()
                .get_content();
            show_popup(s, &name);
        }));

    siv.run();
}

fn show_popup(s: &mut Cursive, name: &str) {
    if name.is_empty() {
        s.add_layer(Dialog::info("Please enter a name!"));
    } else {
        let content = format!("Hello {}!", name);
        s.pop_layer();
        s.add_layer(Dialog::around(TextView::new(content))
            .button("Quit", |s| s.quit()));
    }
}
