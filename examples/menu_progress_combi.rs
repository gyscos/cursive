extern crate cursive;

use cursive::prelude::*;
use cursive::align::HAlign;
use cursive::view::Counter;

fn main() {
    let value = vec![
        Counter::new(0),
        Counter::new(0),
        Counter::new(0),
        Counter::new(0),
        Counter::new(0),
    ];
    let mut siv = Cursive::new();

    let value_menubar = value.clone();

    siv.menubar()
       .add("File", MenuTree::new().leaf("Quit", |s| s.quit()))
       .add("Bar", MenuTree::new().leaf("Increase", move |_| {
           for (i, n) in value_menubar.iter().enumerate() {
               match i {
                   0 => {
                       if n.get() < 100 {
                           n.tick(9);
                       };
                   },
                   1 => {
                       if n.get() < 100 {
                           n.tick(7);
                       };
                   },
                   2 => {
                       if n.get() < 100 {
                           n.tick(10);
                       };
                   },
                   3 => {
                       if n.get() < 100 {
                           n.tick(13);
                       };
                   },
                   4 => {
                       if n.get() < 100 {
                           n.tick(18);
                       };
                   },
                   _ => {},
               }

               if n.get() > 100 {
                   n.set(100);
               }
           }
       }));

    siv.add_layer(Dialog::new(LinearLayout::vertical()
            .child(TextView::new("Title").h_align(HAlign::Center))
            // Box the textview, so it doesn't get too wide.
            // A 0 height value means it will be unconstrained.
            .child(BoxView::fixed_width(60, ProgressBar::new()
                                                .range(0, 100)
                                                .with_value(value[0].clone())))
            .child(BoxView::fixed_width(60, ProgressBar::new()
                                                .range(0, 100)
                                                .with_value(value[1].clone())))
            .child(BoxView::fixed_width(60, ProgressBar::new()
                                                .range(0, 100)
                                                .with_value(value[2].clone())))
            .child(BoxView::fixed_width(60, ProgressBar::new()
                                                .range(0, 100)
                                                .with_value(value[3].clone())))
            .child(BoxView::fixed_width(60, ProgressBar::new()
                                                .range(0, 100)
                                                .with_value(value[4].clone())))));

    // siv.set_autohide_menu(false);
    siv.add_global_callback(Key::Esc, |s| s.select_menubar());

    siv.run();
}

