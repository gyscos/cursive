use utils::LinesIterator;
use style::*;

use unicode_width::UnicodeWidthStr;

/// Generates rows of styled text in constrained width.
///
/// Given styled text and a width constraint, it iterates over
/// collections of styled spans, each making up a row within the constraint.
///
/// # Examples
///
/// ```
/// # use cursive::style::{StyledString, StyledText};
/// # use cursive::utils::StyledLinesIterator;
/// # let text_styled: &[StyledString] = &[].as_ref();
/// let text_plain = text_styled.to_plain();
/// let spans = StyledLinesIterator::make_spans(text_styled);
/// // You can now keep `text_plain` and `spans` alive while using `iter`.
/// let iter = StyledLinesIterator::new(text_plain.as_str(), spans.as_slice(), 10);
/// ```
pub struct StyledLinesIterator<'a> {
    content: &'a [StyledSpan],
    lines_iter: LinesIterator<'a>,

    idx: usize,
    offset: usize
}

impl<'a> StyledLinesIterator<'a> {
    /// Returns the input as `StyledSpan`s so
    /// you can keep them in scope when using a `StyledLinesIterator`.
    pub fn make_spans(text: &'a [StyledString]) -> Vec<StyledSpan> {
        let mut spans = Vec::with_capacity(text.len());

        let mut cut = 0;
        for part in text {
            let (ref str, style) = *part;
            let next_cut = cut + str.len();
            spans.push(StyledSpan {
                style: style,
                start: cut,
                end: next_cut,
                width: str.width()
            });
            cut = next_cut;
        }

        spans
    }

    /// Returns a `StyledLinesIterator` over the given input.
    ///
    /// Get the `plain` text from [`StyledText::to_plain()`](../style/trait.StyledText.html#tymethod.to_plain) and the `styled_spans` from `make_spans()`.
    pub fn new(plain: &'a str, styled_spans: &'a [StyledSpan], width: usize) -> Self {
        StyledLinesIterator {
            content: styled_spans,
            lines_iter: LinesIterator::new(plain, width),
            idx: 0,
            offset: 0
        }
    }

    /// See [`LinesIterator::show_spaces()`](struct.LinesIterator.html#method.show_spaces).
    pub fn show_spaces(mut self) -> Self {
        self.lines_iter = self.lines_iter.show_spaces();
        self
    }
}

/// Represents a row of styled text
/// as a collection of `StyledSpan`s making up the row.
pub type StyledRow = Vec<StyledSpan>;

impl<'a> Iterator for StyledLinesIterator<'a> {
    type Item = StyledRow;

    fn next(&mut self) -> Option<Self::Item> {
        self.lines_iter.next().map(|row| {
            let mut parts = Vec::new();

            let mut current_width = 0;
            while current_width < row.width {
                let current = self.content[self.idx];

                let start = current.start + self.offset;
                let end = if current.end <= row.end {
                    self.offset = 0;
                    self.idx += 1;
                    current.end
                } else {
                    self.offset = row.end - start;
                    row.end
                };
                let width = self.lines_iter.get_content()[start..end].width();

                parts.push(StyledSpan {
                    style: current.style,
                    start: start,
                    end: end,
                    width: width
                });

                current_width += width;
            }

            parts
        })
    }
}

/// Represents a slice of text within a string, akin to `StyledString`.
///
/// The corresponding substring should take `width` cells when printed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StyledSpan {
    pub style: Style,
    pub start: usize,
    pub end: usize,
    pub width: usize
}

#[cfg(test)]
#[test]
fn test() {
    use style::StyledText;
    use std::borrow::Cow;

    let styled = &[
        (Cow::from("1234567_90"), Style::default()),
        (Cow::from("123"), Style { color: Some(::theme::ColorStyle::Highlight), ..Default::default() }),
        (Cow::from("45_"), Style { effect: ::theme::Effect::Reverse, ..Default::default() }),
        (Cow::from("78"), Style::default()),
        (Cow::from("12345_78"), Style::default()),
        (Cow::from("90"), Style { color: Some(::theme::ColorStyle::TitlePrimary), effect: ::theme::Effect::Reverse })
    ];
    let styled = styled.as_ref();

    let plain = styled.to_plain();
    assert_eq!("1234567_9012345_7812345_7890", plain.as_str());
    let spans = StyledLinesIterator::make_spans(styled);
    let mut iter = StyledLinesIterator::new(plain.as_str(), spans.as_slice(), 8);

    let log = |styled_row: &StyledRow| {
        println!("row ({}) {:?}:", styled_row.len(), styled_row);
        for part in styled_row {
           println!("{} {:?}", &plain[part.start..part.end], part);
        }
    };

    { // 12345678
        let styled_row = iter.next().unwrap();
        log(&styled_row);
        assert_eq!(styled_row.len(), 1);

        let part = styled_row[0];
        assert_eq!(part.style, Style::default());
        assert_eq!(part.start, 0);
        assert_eq!(part.end, 8);
        assert_eq!(part.width, 8);
    }

    { // 90 123 456
        let styled_row = iter.next().unwrap();
        log(&styled_row);
        assert_eq!(styled_row.len(), 3);

        let part = styled_row[0];
        assert_eq!(part.style, Style::default());
        assert_eq!(part.start, 8);
        assert_eq!(part.end, 10);
        assert_eq!(part.width, 2);

        let part = styled_row[1];
        assert_eq!(part.style, Style { color: Some(::theme::ColorStyle::Highlight), .. Default::default() });
        assert_eq!(part.start, 10);
        assert_eq!(part.end, 13);
        assert_eq!(part.width, 3);

        let part = styled_row[2];
        assert_eq!(part.style, Style { effect: ::theme::Effect::Reverse, .. Default::default() });
        assert_eq!(part.start, 13);
        assert_eq!(part.end, 16);
        assert_eq!(part.width, 3);
    }

    { // 78 123456
        let styled_row = iter.next().unwrap();
        log(&styled_row);
        assert_eq!(styled_row.len(), 2);

        let part = styled_row[0];
        assert_eq!(part.style, Style::default());
        assert_eq!(part.start, 16);
        assert_eq!(part.end, 18);
        assert_eq!(part.width, 2);

        let part = styled_row[1];
        assert_eq!(part.style, Style::default());
        assert_eq!(part.start, 18);
        assert_eq!(part.end, 24);
        assert_eq!(part.width, 6);
    }

    { // 78 90
        let styled_row = iter.next().unwrap();
        log(&styled_row);
        assert_eq!(styled_row.len(), 2);

        let part = styled_row[0];
        assert_eq!(part.style, Style::default());
        assert_eq!(part.start, 24);
        assert_eq!(part.end, 26);
        assert_eq!(part.width, 2);

        let part = styled_row[1];
        assert_eq!(part.style, Style { color: Some(::theme::ColorStyle::TitlePrimary), effect: ::theme::Effect::Reverse });
        assert_eq!(part.start, 26);
        assert_eq!(part.end, 28);
        assert_eq!(part.width, 2);
    }

    assert_eq!(iter.next(), None);
}
