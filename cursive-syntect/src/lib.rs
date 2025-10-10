//! Parse text using a [`syntect`] highlighter.
//!
//! The [`parse()`] function can be used to generate a StyledString using a
//! highlighter and a syntax set.
//!
//! [`syntect`]: https://docs.rs/syntect
#![deny(missing_docs)]

use cursive_core::style;
use cursive_core::utils::markup::{StyledIndexedSpan, StyledString};
use cursive_core::utils::span::IndexedCow;

use unicode_width::UnicodeWidthStr;

/// Translate a syntect font style into a set of cursive effects.
pub fn translate_effects(font_style: syntect::highlighting::FontStyle) -> style::Effects {
    let mut effects = style::Effects::empty();

    for &(style, effect) in &[
        (syntect::highlighting::FontStyle::BOLD, style::Effect::Bold),
        (
            syntect::highlighting::FontStyle::UNDERLINE,
            style::Effect::Underline,
        ),
        (
            syntect::highlighting::FontStyle::ITALIC,
            style::Effect::Italic,
        ),
    ] {
        if font_style.contains(style) {
            effects.insert(effect);
        }
    }

    effects
}

/// Translate a syntect color into a cursive color.
pub fn translate_color(color: syntect::highlighting::Color) -> style::Color {
    style::Color::Rgb(color.r, color.g, color.b)
}

/// Translate a syntect style into a cursive style.
pub fn translate_style(style: syntect::highlighting::Style) -> style::Style {
    let front = translate_color(style.foreground);
    let back = translate_color(style.background);

    style::Style {
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
