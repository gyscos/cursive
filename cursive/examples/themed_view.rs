use cursive::{views, With};

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
        // Just for this function, import all color names for convenience.
        use cursive::style::{BaseColor::*, PaletteColor::*};

        theme.palette[View] = Black.dark();
        theme.palette[Primary] = Green.light();
        theme.palette[TitlePrimary] = Green.light();
        theme.palette[Highlight] = Green.dark();
        theme.palette[HighlightText] = Green.light();
    });

    // We wrap the `Dialog` inside a `Layer` so it fills the entire view with the new `View` color.
    s.add_layer(views::ThemedView::new(
        theme,
        views::Layer::new(views::Dialog::info("Colors!").title("Themed Dialog")),
    ));
}
