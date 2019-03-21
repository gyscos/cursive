use super::LinesIterator;
use crate::theme::{Effect, Style};
use crate::utils::markup::StyledString;
use crate::utils::span::Span;

fn input() -> StyledString {
    let mut text = StyledString::plain("I ");
    text.append(StyledString::styled("didn't", Effect::Bold));
    text.append(StyledString::plain(" say "));
    text.append(StyledString::styled("half", Effect::Italic));
    text.append(StyledString::plain(" the things people say I did."));
    text.append(StyledString::plain("\n"));
    text.append(StyledString::plain("\n"));
    text.append(StyledString::plain("    - A. Einstein"));

    text
}

#[test]
fn test_line_breaks() {
    let input = input();

    let iter = LinesIterator::new(&input, 17);

    let rows: Vec<_> = iter.map(|row| row.resolve(&input)).collect();

    // println!("{:?}", rows);

    assert_eq!(
        &rows[..],
        &[
            vec![
                Span {
                    content: "I ",
                    attr: &Style::none(),
                    width: 2,
                },
                Span {
                    content: "didn't",
                    attr: &Style::from(Effect::Bold),
                    width: 6,
                },
                Span {
                    content: " say ",
                    attr: &Style::none(),
                    width: 5,
                },
                Span {
                    content: "half",
                    attr: &Style::from(Effect::Italic),
                    width: 4,
                },
            ],
            vec![Span {
                content: "the things people",
                attr: &Style::none(),
                width: 17,
            }],
            vec![Span {
                content: "say I did.",
                attr: &Style::none(),
                width: 10,
            }],
            vec![],
            vec![Span {
                content: "    - A. Einstein",
                attr: &Style::none(),
                width: 17
            }],
        ]
    );
}
