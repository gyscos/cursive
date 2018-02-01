extern crate cursive;

use cursive::Cursive;
use cursive::event::Key;
use cursive::menu::MenuTree;
use cursive::traits::*;
use cursive::view::Position;
use cursive::views::Dialog;
use cursive::views::LayerPosition;
use cursive::views::TextView;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Moves top layer by the specifyed amount
fn move_top(c: &mut Cursive, x_in: isize, y_in: isize) {
    {
        // Step 1. Get the current position of the layer.
        let s = c.screen_mut();
        let l = LayerPosition::FromFront(0);
        let (x, y) = s.offset().pair();

        // Step 2. add the specifed amount
        // (unsigned math in Rust is a mess.)
        let x = if x_in < 0 {
            x - (-x_in) as usize
        } else {
            x + x_in as usize
        };
        let y = if y_in < 0 {
            y - (-y_in) as usize
        } else {
            y + y_in as usize
        };

        // convert the new x and y into a position
        let p = Position::absolute((x, y));

        // Step 3. Apply the new position
        s.reposition_layer(l, p);
    }
    // Step 4. clean the screen cos we made it dirty.
    //c.clear();
}

fn main() {
    let mut siv = Cursive::new();

    // We'll use a counter to name new files.
    let counter = AtomicUsize::new(1);

    // The menubar is a list of (label, menu tree) pairs.
    siv.menubar()
        // We add a new "File" tree
        .add_subtree("File",
             MenuTree::new()
                 // Trees are made of leaves, with are directly actionable...
                 .leaf("New", move |s| {
                     // Here we use the counter to add an entry
                     // in the list of "Recent" items.
                     let i = counter.fetch_add(1, Ordering::Relaxed);
                     let filename = format!("New {}", i);
                     s.menubar().find_subtree("File").unwrap()
                                .find_subtree("Recent").unwrap()
                                .insert_leaf(0, filename, |_| ());

                     s.add_layer(Dialog::info("New file!"));
                 })
                 // ... and of sub-trees, which open up when selected.
                 .subtree("Recent",
                          // The `.with()` method can help when running loops
                          // within builder patterns.
                          MenuTree::new().with(|tree| {
                              for i in 1..100 {
                                  // We don't actually do anything here,
                                  // but you could!
                                  tree.add_leaf(format!("Item {}", i), |_| ())
                              }
                          }))
                 // Delimiter are simple lines between items,
                 // and cannot be selected.
                 .delimiter()
                 .with(|tree| {
                     for i in 1..10 {
                         tree.add_leaf(format!("Option {}", i), |_| ());
                     }
                 }))
        .add_subtree("Help",
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
                       |s| s.add_layer(Dialog::info("Cursive v0.0.0"))))
        .add_delimiter()
        .add_leaf("Quit", |s| s.quit());

    // When `autohide` is on (default), the menu only appears when active.
    // Turning it off will leave the menu always visible.
    // Try uncommenting this line!

    // siv.set_autohide_menu(false);

    siv.add_global_callback(Key::Esc, |s| s.select_menubar());

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);
    siv.add_global_callback('w', |s| move_top(s, 0, -1));
    siv.add_global_callback('a', |s| move_top(s, -1, 0));
    siv.add_global_callback('s', |s| move_top(s, 0, 1));
    siv.add_global_callback('d', |s| move_top(s, 1, 0));

    // Add a simple view
    siv.add_layer(TextView::new(
        "Hit <Esc> to show the menu!\n\n\
         Press w,a,s,d to move the window.\n\
         Press q to quit the application.",
    ));

    // Run the event loop
    siv.run();
}
