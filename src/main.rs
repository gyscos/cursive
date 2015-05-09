extern crate cursive;

use cursive::{Cursive,Dialog};

fn main() {
    let mut siv = Cursive::new();

    siv.new_layer(
        Dialog::new("Hello World !")
            .button("ok", |s| s.quit() ));

    siv.run();
}
