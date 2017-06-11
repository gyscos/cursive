extern crate cursive;

use cursive::Cursive;
use cursive::theme::{ColorStyle, BaseColor, Color, BorderStyle};
use cursive::views::{EditView, LinearLayout, Dialog, TextView};

fn main() {
    let mut siv = Cursive::new();

    let layout = LinearLayout::vertical()
        .child(TextView::new("This is a dynamic theme example!"))
        .child(EditView::new().content("Woo! colors!").style(ColorStyle::Custom {
                                         front: Color::Rgb(200, 150, 150),
                                         back: Color::Dark(BaseColor::Blue),
                                     }));

    siv.add_layer(Dialog::around(layout)
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
