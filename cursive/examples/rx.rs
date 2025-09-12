use cursive::traits::*;
use cursive::views;

fn main() {
    let mut siv = cursive::default();

    let rx = cursive::utils::Rx::new(String::new());
    let text = rx.map(|text| {
        let styled = cursive::utils::markup::cursup::parse(text);
        styled
    });

    // A simple (converging) loop of `Rx`.

    siv.add_layer(
        views::LinearLayout::vertical()
            .child(views::TextView::new(
                "This is an interactive markup editor.\nFor example: /red{Woow}",
            ))
            .child(views::DummyView)
            .child(views::EditView::new().shared_content(rx))
            .child(views::DummyView)
            .child(views::TextView::new_with_content(text).min_height(1))
            .fixed_width(40),
    );

    siv.run();
}
