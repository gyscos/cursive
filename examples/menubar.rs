extern crate cursive;

use cursive::Cursive;
use cursive::event::Key;
use cursive::menu::MenuTree;
use cursive::traits::*;
use cursive::views::Dialog;

fn main() {

    let mut siv = Cursive::new();

    // The menubar is a list of (label, menu tree) pairs.
    siv.menubar()
        // We add a new "File" tree
        .add("File",
             MenuTree::new()
                 // Trees are made of leaves, with are directly actionable...
                 .leaf("New", |s| s.add_layer(Dialog::info("New file!")))
                 // ... and of sub-trees, which open up when selected.
                 .subtree("Recent",
                          // The `.with()` method can help when running loops
                          // within builder patterns.
                          MenuTree::new().with(|tree| {
                              for i in 1..100 {
                                  // We don't actually do anything here,
                                  // but you could!
                                  tree.add_leaf(&format!("Item {}", i), |_| ())
                              }
                          }))
                 // Delimiter are simple lines between items,
                 // and cannot be selected.
                 .delimiter()
                 .with(|tree| {
                     for i in 1..10 {
                         tree.add_leaf(&format!("Option {}", i), |_| ());
                     }
                 })
                 .delimiter()
                 .leaf("Quit", |s| s.quit()))
        .add("Help",
             MenuTree::new()
                 .subtree("Help",
                          MenuTree::new()
                              .leaf("General", |s| {
                                  s.add_layer(Dialog::info("Help message!"))
                              })
                              .leaf("Online", |s| {
                                  let text = "Google it yourself!\n\
                                              Kids, these days...";
                                  s.add_layer(Dialog::info(text))
                              }))
                 .leaf("About",
                       |s| s.add_layer(Dialog::info("Cursive v0.0.0"))));

    // When `autohide` is on (default), the menu only appears when it is active.
    // Turning it off will leave the menu always visible.

    // siv.set_autohide_menu(false);

    siv.add_global_callback(Key::Esc, |s| s.select_menubar());

    siv.add_layer(Dialog::text("Hit <Esc> to show the menu!"));

    siv.run();
}
