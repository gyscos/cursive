//! Parse text using a syntect highlighter.
#![deny(missing_docs)]

use cursive_core::theme;
use cursive_core::utils::markup::{StyledIndexedSpan, StyledString};
use cursive_core::utils::span::IndexedCow;

use cursive_core::reexports::enumset::EnumSet;

use unicode_width::UnicodeWidthStr;

/// Translate a syntect font style into a set of cursive effects.
pub fn translate_effects(
    font_style: syntect::highlighting::FontStyle,
) -> EnumSet<theme::Effect> {
    let mut effects = EnumSet::new();

    for &(style, effect) in &[
        (syntect::highlighting::FontStyle::BOLD, theme::Effect::Bold),
        (
            syntect::highlighting::FontStyle::UNDERLINE,
            theme::Effect::Underline,
        ),
        (
            syntect::highlighting::FontStyle::ITALIC,
            theme::Effect::Italic,
        ),
    ] {
        if font_style.contains(style) {
            effects.insert(effect);
        }
    }

    effects
}

/// Translate a syntect color into a cursive color.
pub fn translate_color(color: syntect::highlighting::Color) -> theme::Color {
    theme::Color::Rgb(color.r, color.g, color.b)
}

/// Translate a syntect style into a cursive style.
pub fn translate_style(style: syntect::highlighting::Style) -> theme::Style {
    let front = translate_color(style.foreground);
    let back = translate_color(style.background);

    theme::Style {
        color: (front, back).into(),
        effects: translate_effects(style.font_style),
    }
}

/// Parse text using a syntect highlighter.
pub fn parse<S: Into<String>>(
    input: S,
    highlighter: &mut syntect::easy::HighlightLines,
    syntax_set: &syntect::parsing::SyntaxSet,
) -> Result<StyledString, syntect::Error> {
    let input = input.into();
    let mut spans = Vec::new();

    for line in input.split_inclusive('\n') {
        for (style, text) in highlighter.highlight_line(line, syntax_set)? {
            spans.push(StyledIndexedSpan {
                content: IndexedCow::from_str(text, &input),
                attr: translate_style(style),
                width: text.width(),
            });
        }
    }

    Ok(StyledString::with_spans(input, spans))
}
