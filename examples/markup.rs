extern crate cursive;

use cursive::Cursive;
#[cfg(feature = "markdown")]
use cursive::utils::markup::markdown;
use cursive::views::{Dialog, TextView};

// Make sure you compile with the `markdown` feature!
//
// cargo run --example markup --features markdown

fn main() {
    let mut siv = Cursive::new();

    // If markdown is enabled, parse a small text.

    #[cfg(feature = "markdown")]
    let text = markdown::parse("Isn't *that* **cool**?");

    #[cfg(not(feature = "markdown"))]
    let text = "Rebuild with --features markdown ;)";

    // TextView can natively accept StyledString.
    siv.add_layer(
        Dialog::around(TextView::new(text)).button("Hell yeah!", |s| s.quit()),
    );

    siv.run();
}
