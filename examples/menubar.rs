extern crate cursive;

use cursive::Cursive;
use cursive::menu::MenuTree;
use cursive::view::Dialog;
use cursive::view::TextView;
use cursive::event::Key;

fn main() {

    let mut siv = Cursive::new();

    siv.menubar()
       .add("File",
            MenuTree::new()
                .leaf("New", |s| s.add_layer(Dialog::info("New file!")))
                .leaf("Quit", |s| s.quit()))
       .add("Help",
            MenuTree::new()
                .leaf("Help", |s| s.add_layer(Dialog::info("Help message!")))
                .leaf("About",
                      |s| s.add_layer(Dialog::info("Cursive v0.0.0"))));

    // siv.set_autohide_menu(false);

    siv.add_global_callback(Key::F(10), |s| s.select_menubar());

    siv.add_layer(Dialog::new(TextView::new("Hit <F10> to show the menu!")));

    siv.run();
}
