use super::LinesIterator;
use theme::{Effect, Style};
use utils::markup::StyledString;
use utils::span::Span;

fn input() -> StyledString {
    let mut text = StyledString::plain("I ");
    text.append(StyledString::styled("didn't", Effect::Bold));
    text.append(StyledString::plain(" say "));
    text.append(StyledString::styled("half", Effect::Italic));
    text.append(StyledString::plain(" the things people say I did."));
    text.append(StyledString::plain("\n"));
    text.append(StyledString::plain("    - A. Einstein"));

    text
}

#[test]
fn test_line_breaks() {
    let input = input();

    let iter = LinesIterator::new(&input, 17);

    let rows: Vec<_> = iter.map(|row| row.resolve(input.as_spanned_str())).collect();

    assert_eq!(
        &rows[..],
        &[
            vec![
                Span {
                    content: "I ",
                    attr: &Style::none(),
                },
                Span {
                    content: "didn't",
                    attr: &Style::from(Effect::Bold),
                },
                Span {
                    content: " say ",
                    attr: &Style::none(),
                },
                Span {
                    content: "half",
                    attr: &Style::from(Effect::Italic),
                },
            ],
            vec![
                Span {
                    content: "the things people",
                    attr: &Style::none(),
                },
            ],
            vec![
                Span {
                    content: "say I did.",
                    attr: &Style::none(),
                },
            ],
            vec![
                Span {
                    content: "    - A. Einstein",
                    attr: &Style::none(),
                },
            ],
        ]
    );
}
