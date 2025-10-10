use crossterm::style::Stylize;
use cursive_core::style::BaseColor::{Black, Red};
use cursive_core::style::ColorStyle;
use cursive_core::theme::Style;
use cursive_core::utils::markup::{cursup, StyledString};
use cursive_core::views::{Dialog, LinearLayout, TextView};

fn main() {
    // Coloring text.
    let crossterm_colored = crossterm_coloring_into_ansi("Crossterm colored text.");

    let cursive_parsed_ansi = cursive_parse_ansi("Parsed ANSI");

    let cursive_single_span = single_style_span("Cursive StyledString::single_span");

    let cursup_markup = cursup_markup("/blue+bold{Cursup} /yellow+bold{markup}");

    // Removing styles.

    let cleared_by_iterating: String = iterating_spans(&cursive_parsed_ansi);

    let cleared_by_canonicalize: String = canonicalize_clear(&cursive_single_span);

    // Minimal application for text output
    let mut siv = cursive::default();
    siv.add_layer(
        Dialog::new()
            .content(
                LinearLayout::vertical()
                    .child(TextView::new(crossterm_colored))
                    .child(TextView::new(cursive_parsed_ansi))
                    .child(TextView::new(cursive_single_span))
                    .child(TextView::new(cursup_markup))
                    .child(TextView::new(cleared_by_iterating))
                    .child(TextView::new(cleared_by_canonicalize)),
            )
            .button("Quit!", |s| s.quit()),
    );
    siv.run();
}

// Crossterm function for text styling and casting it to a string.
// Results in raw text with ANSI codes.
fn crossterm_coloring_into_ansi(str: &str) -> String {
    str.red().to_string()
}

// Parsing ansi into StyledString
// https://docs.rs/cursive/latest/cursive/utils/markup/ansi/fn.parse.html
fn cursive_parse_ansi(str: &str) -> StyledString {
    cursive::utils::markup::ansi::parse(str.red().to_string())
}

// Building cursive-native StyledString with StyledString::single_span (single style for entire string)
// https://docs.rs/cursive/latest/cursive/utils/markup/type.StyledString.html#method.single_span
fn single_style_span(str: &str) -> StyledString {
    StyledString::single_span(
        str,
        Style {
            effects: Default::default(),
            color: ColorStyle::new(Red, Black),
        },
    )
}

// Cursup, a simple markup language.
// https://docs.rs/cursive/latest/cursive/utils/markup/cursup/index.html
fn cursup_markup(str: &str) -> StyledString {
    cursup::parse(str)
}

// Iterating on the spans, and accumulating the span text content, ignoring the styles.
fn iterating_spans(str: &StyledString) -> String {
    str.spans().map(|span| span.content).collect()
}

// Use compact or canonicalize, which rebuilds the source to only include visible text (no markup), and then get the source.
// Note that this technically modifies the StyledString, and requires mutable access.
// https://docs.rs/cursive/latest/cursive/utils/span/struct.SpannedString.html#method.canonicalize
// https://docs.rs/cursive/latest/cursive/utils/span/struct.SpannedString.html#method.compact
fn canonicalize_clear(str: &StyledString) -> String {
    let mut canonicalize_clear = str.clone();
    canonicalize_clear.canonicalize();
    canonicalize_clear.source().to_string()
}
