extern crate cursive;

use cursive::Cursive;
use cursive::view::{TextView,Dialog};

fn main() {
    let mut siv = Cursive::new();

    // Creates a dialog with a single "Quit" button
    siv.add_layer(Dialog::new(TextView::new("Hello Dialog!"))
                    .button("Quit", |s,_| s.quit()));

    siv.run();
}
