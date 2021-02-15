use cursive::{self, theme, views, With};

fn main() {
    let mut cursive = cursive::default();

    cursive.add_layer(
        views::Dialog::text("Open a themed dialog?")
            .button("Open", show_dialog)
            .button("Quit", |s| s.quit()),
    );

    cursive.run();
}

fn show_dialog(s: &mut cursive::Cursive) {
    // Let's build a green theme
    let theme = s.current_theme().clone().with(|theme| {
        theme.palette[theme::PaletteColor::View] =
            theme::Color::Dark(theme::BaseColor::Black);
        theme.palette[theme::PaletteColor::Primary] =
            theme::Color::Light(theme::BaseColor::Green);
        theme.palette[theme::PaletteColor::TitlePrimary] =
            theme::Color::Light(theme::BaseColor::Green);
        theme.palette[theme::PaletteColor::Highlight] =
            theme::Color::Dark(theme::BaseColor::Green);
    });

    s.add_layer(views::ThemedView::new(
        theme,
        views::Layer::new(
            views::Dialog::info("Colors!").title("Themed Dialog"),
        ),
    ));
}
