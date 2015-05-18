extern crate cursive;

use cursive::Cursive;
use cursive::view::TextView;

use std::fs::File;
use std::io::Read;

fn main() {
    let mut siv = Cursive::new();

    // Read some long text to showcase the layout
    let mut file = File::open("assets/lorem.txt").unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    // We can quit by pressing q
    siv.add_global_callback('q' as i32, |s,_| s.quit());

    siv.add_layer(TextView::new(&content));

    siv.run();
}

