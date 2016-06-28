extern crate cursive;

use cursive::Cursive;
use cursive::view::Dialog;
use cursive::event::Key;

fn main() {

    let mut siv = Cursive::new();

    siv.menu()
       .new_subtree("File")
       .leaf("New", |s| s.add_layer(Dialog::info("New file!")))
       .leaf("Quit", |s| s.quit());

    siv.menu()
       .new_subtree("Help")
       .leaf("Help", |s| s.add_layer(Dialog::info("Help message!")))
       .leaf("About", |s| s.add_layer(Dialog::info("Cursive v0.0.0")));

    siv.add_global_callback(Key::F(10), |s| s.select_menu());

    siv.run();
}
