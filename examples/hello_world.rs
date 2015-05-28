extern crate cursive;

use cursive::Cursive;
use cursive::view::TextView;

fn main() {
    let mut siv = Cursive::new();

    // We can quit by pressing q
    siv.add_global_callback('q', |s| s.quit());

    siv.add_layer(TextView::new("Hello World!\nPress q to quit the application."));

    siv.run();
}
