use cursive::style::{BorderStyle, Palette};
use cursive::traits::With;
use cursive::views::{Dialog, EditView, LinearLayout, TextView};
use cursive::Cursive;

fn main() {
    let mut siv = cursive::default();

    // Start with a nicer theme than default
    siv.set_theme(cursive::theme::Theme {
        shadow: true,
        borders: BorderStyle::Simple,
        palette: Palette::retro().with(|palette| {
            use cursive::style::BaseColor::*;

            {
                // First, override some colors from the base palette.
                use cursive::style::Color::TerminalDefault;
                use cursive::style::PaletteColor::*;

                palette[Background] = TerminalDefault;
                palette[View] = TerminalDefault;
                palette[Primary] = White.dark();
                palette[TitlePrimary] = Blue.light();
                palette[Secondary] = Blue.light();
                palette[Highlight] = Blue.dark();
            }

            {
                // Then override some styles.
                use cursive::style::Effect::*;
                use cursive::style::PaletteStyle::*;
                use cursive::style::Style;
                palette[Highlight] = Style::from(Blue.light()).combine(Bold);
                palette[EditableTextCursor] = Style::secondary().combine(Reverse).combine(Underline)
            }
        }),
    });

    let layout = LinearLayout::vertical()
        .child(TextView::new("This is a dynamic theme example!"))
        .child(EditView::new().content("Woo! colors!").filler(" "));

    siv.add_layer(
        Dialog::around(layout)
            .title("Theme example")
            .button("Change", |s| {
                use cursive::style::BaseColor::*;
                use cursive::style::Color::TerminalDefault;
                use cursive::style::PaletteColor::*;
                // Change _something_ when the button is pressed.
                let mut theme = s.current_theme().clone();

                theme.shadow = !theme.shadow;
                theme.borders = match theme.borders {
                    BorderStyle::None => {
                        theme.palette[View] = TerminalDefault;
                        BorderStyle::Simple
                    }
                    _ => {
                        theme.palette[View] = Black.light();
                        BorderStyle::None
                    }
                };

                s.set_theme(theme);
            })
            .button("Quit", Cursive::quit),
    );

    siv.run();
}
