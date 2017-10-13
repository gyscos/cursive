extern crate cursive;

use cursive::Cursive;
use cursive::traits::*;
use cursive::views::{Dialog, TextArea};

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(
        Dialog::new()
            .title("Describe your issue")
            .padding((1, 1, 1, 0))
            .content(TextArea::new().with_id("text"))
            .button("Ok", Cursive::quit),
    );

    siv.run();
}
