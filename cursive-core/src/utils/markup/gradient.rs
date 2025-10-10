//! Generate StyledString with gradients.

use crate::style::{gradient::Linear, ColorStyle, Rgb, Style};
use crate::utils::markup::{StyledIndexedSpan, StyledString};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Generate spans for the given styled string to re-color the back.
pub fn decorate_back_with<F>(text: &StyledString, mut colorization: F) -> Vec<StyledIndexedSpan>
where
    F: FnMut(f32) -> Rgb,
{
    decorate_with(text, |old_style, x| {
        // Combine color with the current style
        old_style.combine(ColorStyle::back(colorization(x).as_color()))
    })
}

/// Generate spans for the given styled string to re-color the front.
pub fn decorate_front_with<F>(text: &StyledString, mut colorization: F) -> Vec<StyledIndexedSpan>
where
    F: FnMut(f32) -> Rgb,
{
    decorate_with(text, |old_style, x| {
        // Combine color with the current style
        old_style.combine(ColorStyle::front(colorization(x).as_color()))
    })
}

/// Generate spans to decorate the given styled string.
pub fn decorate_with<F>(text: &StyledString, mut style_maker: F) -> Vec<StyledIndexedSpan>
where
    F: FnMut(Style, f32) -> Style,
{
    let mut result = Vec::new();
    let mut x = 0f32;

    let mut graphemes = text
        .spans()
        .flat_map(|span| span.content.graphemes(true))
        .map(|g| g.width());
    let first_half = graphemes.next().unwrap_or(0) as f32 / 2f32;
    let last_half = graphemes.next_back().unwrap_or(0) as f32 / 2f32;
    let total_width =
        text.spans().map(|span| span.width).sum::<usize>() as f32 - first_half - last_half;

    for span in text.spans_raw() {
        let mut cursor = 0;
        let text = span.resolve(text.source());
        for g in text.content.graphemes(true) {
            let l = g.len();
            let gw = g.width();
            let gwf = gw as f32;
            let new_color = style_maker(span.attr, (x + (gwf / 2f32) - first_half) / total_width);
            result.push(StyledIndexedSpan {
                content: span.content.subcow(cursor..cursor + l),
                attr: new_color,
                width: gw,
            });
            x += gwf;
            cursor += l;
        }
    }

    result
}

/// Decorate a text with the given gradient.
pub fn decorate_front<S, G>(text: S, gradient: G) -> StyledString
where
    S: Into<StyledString>,
    G: Into<Linear>,
{
    let text = text.into();
    let gradient = gradient.into();

    let spans = decorate_front_with(&text, |x| gradient.interpolate(x).as_u8());

    StyledString::with_spans(text.into_source(), spans)
}

/// Decorate a text with the given gradient as background.
pub fn decorate_back<S, G>(text: S, gradient: G) -> StyledString
where
    S: Into<StyledString>,
    G: Into<Linear>,
{
    let text = text.into();
    let gradient = gradient.into();

    let spans = decorate_back_with(&text, |x| gradient.interpolate(x).as_u8());

    StyledString::with_spans(text.into_source(), spans)
}

#[cfg(test)]
mod tests {
    use super::decorate_front;
    use crate::style::Rgb;
    use crate::utils::markup::cursup;

    #[test]
    fn simple() {
        let gradient = decorate_front("ab", (Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)));

        assert_eq!(
            gradient,
            cursup::parse("/#FF0000{a}/#0000FF{b}").canonical()
        );

        let gradient = decorate_front("abcde", (Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)));

        assert_eq!(
            gradient,
            cursup::parse("/#FF0000{a}/#BF0040{b}/#800080{c}/#4000Bf{d}/#0000FF{e}").canonical()
        );
    }
}
