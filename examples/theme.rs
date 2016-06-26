extern crate cursive;

use cursive::Cursive;
use cursive::view::{Dialog, TextView};

fn main() {
    let mut siv = Cursive::new();
    siv.load_theme("assets/style.toml");

    siv.add_layer(Dialog::new(TextView::new("This application uses a custom theme!"))
                      .title("Themed dialog")
                      .button("Quit", |s| s.quit()));

    siv.run();
}
