//! This example creates a status bar at the bottom of the screen.

use cursive::{
    style::BaseColor,
    traits::{Nameable as _, Resizable as _},
    utils::markup::StyledString,
    view::View as _,
    views::{Dialog, FixedLayout, Layer, OnLayoutView, TextView},
    {Rect, Vec2},
};

fn main() {
    let mut siv = cursive::default();

    // To build the status bar, we use a full-screen transparent layer.
    // We use FixedLayout (with OnLayoutView) to manually position our
    // TextView at the bottom of the screen.
    siv.screen_mut().add_transparent_layer(
        OnLayoutView::new(
            FixedLayout::new().child(
                Rect::from_point(Vec2::zero()),
                Layer::new(TextView::new("Status: unknown").with_name("status")).full_width(),
            ),
            |layout, size| {
                // We could also keep the status bar at the top instead.
                layout.set_child_position(0, Rect::from_size((0, size.y - 1), (size.x, 1)));
                layout.layout(size);
            },
        )
        .full_screen(),
    );

    // We'll add a single dialog with a Quit button, and another button
    // that changes the status.
    siv.add_layer(
        Dialog::new()
            .title("Status bar example")
            .button("Change status", |s| {
                s.call_on_name("status", |text: &mut TextView| {
                    // Flip the current situation.
                    let nominal = !text.get_content().source().contains("nominal");

                    text.set_content(make_message(nominal));
                })
                .unwrap();
            })
            .button("Quit", |s| s.quit()),
    );

    siv.run();
}

/// Prepare a colorful message based on the status.
fn make_message(nominal: bool) -> StyledString {
    let mut status = StyledString::plain("Status: ");

    if nominal {
        status.append_styled("nominal", BaseColor::Green.dark());
    } else {
        status.append_styled("error", BaseColor::Red.dark());
    }

    status
}
