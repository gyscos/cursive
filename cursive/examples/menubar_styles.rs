use cursive::style::{BaseColor, Color, Effect, Style};
use cursive::utils::markup::StyledString;
use cursive::{menu, traits::*};

fn main() {
    let mut siv = cursive::default();

    let mut styles_label = StyledString::plain("");
    styles_label.append(StyledString::styled("S", Color::Dark(BaseColor::Red)));
    styles_label.append(StyledString::styled("t", Color::Dark(BaseColor::Green)));
    styles_label.append(StyledString::styled("y", Color::Dark(BaseColor::Yellow)));
    styles_label.append(StyledString::styled("l", Color::Dark(BaseColor::Blue)));
    styles_label.append(StyledString::styled("e", Color::Dark(BaseColor::Magenta)));
    styles_label.append(StyledString::styled("s", Color::Dark(BaseColor::Cyan)));

    let quit_label = StyledString::styled(
        "Quit",
        Style::from(Color::Dark(BaseColor::Red)).combine(Effect::Bold),
    );

    let sub_item_labels = vec![
        StyledString::styled("Black", Color::Dark(BaseColor::Black)),
        StyledString::styled("Red", Color::Dark(BaseColor::Red)),
        StyledString::styled("Green", Color::Dark(BaseColor::Green)),
        StyledString::styled("Yellow", Color::Dark(BaseColor::Yellow)),
        StyledString::styled("Blue", Color::Dark(BaseColor::Blue)),
        StyledString::styled("Magenta", Color::Dark(BaseColor::Magenta)),
        StyledString::styled("Cyan", Color::Dark(BaseColor::Cyan)),
        StyledString::styled("White", Color::Dark(BaseColor::White)),
        StyledString::styled("Light Black", Color::Light(BaseColor::Black)),
        StyledString::styled("Light Red", Color::Light(BaseColor::Red)),
        StyledString::styled("Light Green", Color::Light(BaseColor::Green)),
        StyledString::styled("Light Yellow", Color::Light(BaseColor::Yellow)),
        StyledString::styled("Light Blue", Color::Light(BaseColor::Blue)),
        StyledString::styled("Light Magenta", Color::Light(BaseColor::Magenta)),
        StyledString::styled("Light Cyan", Color::Light(BaseColor::Cyan)),
        StyledString::styled("Light White", Color::Light(BaseColor::White)),
    ];

    siv.menubar()
        .add_subtree(
            styles_label,
            menu::Tree::new().with(|tree| {
                for label in &sub_item_labels {
                    tree.add_leaf(label.clone(), |_| {});
                }
            }),
        )
        .add_delimiter()
        .add_leaf(quit_label, |s| s.quit());

    siv.set_autohide_menu(false);

    siv.run();
}
