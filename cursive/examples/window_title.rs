fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        cursive::views::Dialog::new()
            .title("Write yourself a new title!")
            .content(cursive::views::EditView::new().on_edit(
                |s, content, _| {
                    s.set_window_title(content);
                },
            ))
            .button("Quit", |s| s.quit()),
    );

    siv.run();
}
