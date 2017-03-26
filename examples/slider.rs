extern crate cursive;

use cursive::Cursive;
use cursive::traits::*;
use cursive::views::{Dialog, SliderView};

fn main() {
    let mut siv = Cursive::new();

    // Let's add a simple slider in a dialog.
    // Moving the slider will update the dialog's title.
    // And pressing "Enter" will show a new dialog.
    siv.add_layer(Dialog::around(SliderView::horizontal(15)
            .value(7)
            .on_change(|s, v| {
                let title = format!("[ {} ]", v);
                s.call_on_id("dialog", |view: &mut Dialog| view.set_title(title));
            })
            .on_enter(|s, v| {
                s.pop_layer();
                s.add_layer(Dialog::text(format!("Lucky number {}!", v))
                    .button("Ok", Cursive::quit));
            }))
        .title("[ 7 ]")
        .with_id("dialog"));

    siv.run();
}
