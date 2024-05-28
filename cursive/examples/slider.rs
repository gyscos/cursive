use cursive::traits::*;
use cursive::views::{Dialog, SliderView};
use cursive::Cursive;

fn main() {
    let mut siv = cursive::default();

    siv.add_global_callback('q', |s| s.quit());

    // Let's add a simple slider in a dialog.
    // Moving the slider will update the dialog's title.
    // And pressing "Enter" will show a new dialog.
    siv.add_layer(
        Dialog::around(
            // We give the number of steps in the constructor
            SliderView::horizontal(15)
                // Sets the initial value
                .value(7)
                .on_change(|s, v| {
                    let title = format!("{v: ^5}");
                    s.call_on_name("dialog", |view: &mut Dialog| {
                        view.set_title(title)
                    });
                })
                .on_enter(|s, v| {
                    s.pop_layer();
                    s.add_layer(
                        Dialog::text(format!("Lucky number {v}!"))
                            .button("Ok", Cursive::quit),
                    );
                }),
        )
        .title("  7  ")
        .with_name("dialog"),
    );

    siv.run();
}
