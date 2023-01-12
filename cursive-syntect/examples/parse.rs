use cursive::traits::*;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use std::rc::Rc;

struct State {
    syntax_set: SyntaxSet,
    themes: ThemeSet,
}

fn main() {
    let mut siv = cursive::default();

    // Load syntect syntax, theme, ...
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let themes = ThemeSet::load_defaults();

    siv.set_user_data(Rc::new(State { syntax_set, themes }));

    // Use it as a single view.
    siv.add_fullscreen_layer(
        cursive::views::TextView::new("")
            .with_name("content")
            .scrollable()
            .full_screen(),
    );

    siv.with_theme(|t| {
        t.shadow = false;
    });

    apply_theme(&mut siv, "InspiredGitHub");

    siv.add_global_callback('q', |s| s.quit());

    siv.add_global_callback('t', |s| {
        let theme_names: Vec<_> = s
            .with_user_data(|s: &mut Rc<State>| s.themes.themes.keys().cloned().collect())
            .unwrap();

        s.add_layer(
            cursive::views::OnEventView::new(
                cursive::views::Dialog::new()
                    .title("Select a theme")
                    .content(
                        cursive::views::SelectView::new()
                            .with_all_str(theme_names)
                            .on_submit(|s, theme_name| {
                                apply_theme(s, theme_name);
                                s.pop_layer();
                            }),
                    ),
            )
            .on_event(cursive::event::Key::Esc, |s| {
                s.pop_layer();
            }),
        );
    });

    siv.add_layer(cursive::views::Dialog::info(
        r"This is a syntect example.

This very file is printed here.

Press T to change the theme.
Press Q to quit.",
    ));

    siv.run();
}

fn apply_theme(siv: &mut cursive::Cursive, theme_name: &str) {
    let state = siv
        .with_user_data(|s: &mut Rc<State>| Rc::clone(s))
        .unwrap();

    let theme = &state.themes.themes[theme_name];
    let syntax = state.syntax_set.find_syntax_by_token("rs").unwrap();
    let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);

    // Apply some settings from the theme to cursive's own theme.
    siv.with_theme(|t| {
        if let Some(background) = theme
            .settings
            .background
            .map(cursive_syntect::translate_color)
        {
            t.palette[cursive::theme::PaletteColor::Background] = background;
            t.palette[cursive::theme::PaletteColor::View] = background;
        }
        if let Some(foreground) = theme
            .settings
            .foreground
            .map(cursive_syntect::translate_color)
        {
            t.palette[cursive::theme::PaletteColor::Primary] = foreground;
            t.palette[cursive::theme::PaletteColor::TitlePrimary] = foreground;
        }

        if let Some(highlight) = theme
            .settings
            .highlight
            .map(cursive_syntect::translate_color)
        {
            t.palette[cursive::theme::PaletteColor::Highlight] = highlight;
        }
    });

    // Read some content somewhere
    let content = include_str!("parse.rs");

    // Parse the content and highlight it
    let styled = cursive_syntect::parse(content, &mut highlighter, &state.syntax_set).unwrap();

    siv.call_on_name("content", |t: &mut cursive::views::TextView| {
        t.set_content(styled);
    })
    .unwrap();
}
