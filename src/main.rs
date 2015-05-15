extern crate cursive;

use cursive::Cursive;
use cursive::view::TextView;

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(TextView::new("Hello World!"));

    // We can quit by pressing q
    siv.add_global_callback('q' as i32, |s| s.quit());

    siv.run();
}
