use cursive::style::{Color, PaletteColor};
use cursive::theme::Theme;
use cursive::views::TextView;
use cursive::Cursive;

// This example sets the background color to the terminal default.
//
// This way, it looks more natural.

fn main() {
    let mut siv = cursive::default();

    let theme = custom_theme_from_cursive(&siv);
    siv.set_theme(theme);

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    siv.add_layer(TextView::new(
        "Hello World with default terminal background color!\n\
         Press q to quit the application.",
    ));

    siv.run();
}

fn custom_theme_from_cursive(siv: &Cursive) -> Theme {
    // We'll return the current theme with a small modification.
    let mut theme = siv.current_theme().clone();

    theme.palette[PaletteColor::Background] = Color::TerminalDefault;

    theme
}
