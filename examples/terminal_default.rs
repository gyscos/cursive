extern crate cursive;

use cursive::Cursive;
use cursive::theme::{Color, Theme};
use cursive::views::TextView;

fn custom_theme_from_cursive(siv: &Cursive) -> Theme {
    let mut theme = siv.current_theme().clone();
    theme.colors.background = Color::TerminalDefault;
    theme
}

fn main() {
    let mut siv = Cursive::new();
    let theme = custom_theme_from_cursive(&siv);

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);
    siv.set_theme(theme);

    siv.add_layer(TextView::new(
        "Hello World with default terminal background color!\n\
         Press q to quit the application.",
    ));

    siv.run();
}
