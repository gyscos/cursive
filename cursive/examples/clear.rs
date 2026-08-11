use cursive::views::TextView;
use cursive::theme::BaseColor;
use cursive::theme::Color;
use cursive::theme::PaletteColor;
use cursive::view::Resizable;
use cursive::{Cursive, View};
use std::process::Command;
use std::env;

fn open_editor(cursive: &mut Cursive) {
    let editor = env::var_os("EDITOR").unwrap_or_else(|| "vim".into());
    let result = Command::new(&editor)
        .spawn()
        .expect("Cannot spawn editor")
        .wait()
        .unwrap();
}

fn main() {
    let mut siv = cursive::default();

    let mut theme = siv.current_theme().clone();;
    theme.palette[PaletteColor::Background] = Color::TerminalDefault;
    theme.palette[PaletteColor::View] = Color::TerminalDefault;
    theme.palette[PaletteColor::Primary] = Color::TerminalDefault;
    theme.palette[PaletteColor::Highlight] = Color::Light(BaseColor::Cyan);
    theme.palette[PaletteColor::HighlightText] = Color::Dark(BaseColor::Black);
    theme.shadow = false;
    siv.set_theme(theme);

    siv.add_global_callback('q', Cursive::quit);
    siv.add_global_callback('e', open_editor);
    siv.add_global_callback('c', Cursive::clear);

    // Add a simple view
    siv.add_fullscreen_layer(TextView::new(
        "Hello World!\n\
         Press q to quit the application.\n\
         Press c to clear the screen.\n\
         Press e to spawn the editor.",
    ).center().full_screen());

    // Run the event loop
    siv.run();
}
