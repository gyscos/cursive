use cursive::traits::*;
use cursive::views::{Button, LinearLayout, Panel};

fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        LinearLayout::vertical()
            .child(
                Panel::new(Button::new("Quit", |s| s.quit()))
                    .title("Panel 1")
                    .fixed_width(20),
            )
            .child(
                Panel::new(Button::new("Quit", |s| s.quit()))
                    .title("Panel 2")
                    .fixed_width(15),
            )
            .child(
                Panel::new(Button::new("Quit", |s| s.quit()))
                    .title("Panel 3")
                    .fixed_width(10),
            )
            .child(
                Panel::new(Button::new("Quit", |s| s.quit()))
                    .title("Panel 4")
                    .fixed_width(5),
            ),
    );

    siv.run();
}
