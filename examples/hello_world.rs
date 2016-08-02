extern crate cursive;

use cursive::prelude::*;

fn main() {
    let mut siv = Cursive::new();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    siv.add_layer(TextView::new("Hello World!\n\
                                Press q to quit the application."));

    siv.run();
}
