extern crate cursive;

use cursive::Cursive;
use cursive::utils::markup::MarkdownText;
use cursive::views::{Dialog, TextView};

// Make sure you compile with the `markdown` feature!
//
// cargo run --example markup --features markdown

fn main() {
    let mut siv = Cursive::new();

    let text = MarkdownText("Isn't *that* **cool**?");

    siv.add_layer(
        Dialog::around(TextView::styled(text).unwrap())
            .button("Hell yeah!", |s| s.quit()),
    );

    siv.run();
}
