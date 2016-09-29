extern crate cursive;

use cursive::Cursive;
use cursive::views::Dialog;
use cursive::theme::BorderStyle;

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Dialog::text("This is a dynamic theme example!")
        .button("Change", |s| {
            let mut theme = s.current_theme().clone();

            theme.shadow = !theme.shadow;
            theme.borders = match theme.borders {
                Some(BorderStyle::Simple) => Some(BorderStyle::Outset),
                Some(BorderStyle::Outset) => None,
                None => Some(BorderStyle::Simple),
            };

            s.set_theme(theme);
        })
        .button("Quit", Cursive::quit));

    siv.run();
}
