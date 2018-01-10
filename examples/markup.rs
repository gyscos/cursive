extern crate cursive;

use cursive::Cursive;
#[cfg(feature = "markdown")]
use cursive::utils::markup::MarkdownText;
use cursive::views::{Dialog, TextView};

// Make sure you compile with the `markdown` feature!
//
// cargo run --example markup --features markdown

fn main() {
    let mut siv = Cursive::new();

    #[cfg(feature = "markdown")]
    let text = MarkdownText("Isn't *that* **cool**?");

    #[cfg(not(feature = "markdown"))]
    let text = "Rebuild with --features markdown ;)";

    siv.add_layer(
        Dialog::around(TextView::styled(text).unwrap())
            .button("Hell yeah!", |s| s.quit()),
    );

    siv.run();
}
