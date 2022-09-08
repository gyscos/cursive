use cursive::traits::Resizable as _;
use cursive::traits::Scrollable as _;

fn main() {
    let mut siv = cursive::default();

    // Load syntect syntax, theme, ...
    let syntax_set = syntect::parsing::SyntaxSet::load_defaults_newlines();

    let ts = syntect::highlighting::ThemeSet::load_defaults();
    let theme = &ts.themes["InspiredGitHub"];
    let syntax = syntax_set.find_syntax_by_token("rs").unwrap();
    let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);

    // Use the theme background color as the app background color
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
    let styled =
        cursive_syntect::parse(content, &mut highlighter, &syntax_set)
            .unwrap();

    // Use it as a single view.
    siv.add_fullscreen_layer(
        cursive::views::TextView::new(styled)
            .scrollable()
            .full_screen(),
    );

    siv.add_layer(cursive::views::Dialog::info(
        "This is a syntect example.\n\nThis very file is printed here.",
    ));

    siv.run();
}
