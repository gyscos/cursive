use cursive::{
    views::{Button, FixedLayout, TextView},
    Rect,
};
fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        cursive::views::Dialog::around(
            FixedLayout::new()
                .child(Rect::from_size((0, 0), (1, 1)), TextView::new("/"))
                .child(Rect::from_size((14, 0), (1, 1)), TextView::new(r"\"))
                .child(Rect::from_size((0, 2), (1, 1)), TextView::new(r"\"))
                .child(Rect::from_size((14, 2), (1, 1)), TextView::new("/"))
                .child(
                    Rect::from_size((2, 1), (11, 1)),
                    Button::new("Click me!", |s| s.quit()),
                ),
        )
        .button("Quit", |s| s.quit()),
    );

    siv.run();
}
