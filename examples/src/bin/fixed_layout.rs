fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        cursive::views::Dialog::around(
            cursive::views::FixedLayout::new().child(
                cursive::Rect::from_size((0, 0), (10, 1)),
                cursive::views::TextView::new("abc"),
            ),
        )
        .button("Quit", |s| s.quit()),
    );

    siv.run();
}
