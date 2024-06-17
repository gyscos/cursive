//! A simple markup format.
//!
//! # Examples
//!
//! ```
//! /red{This} /green{text} /blue{is} /bold{very} /underline{styled}!
//! /red+bold{This too!}
//! ```
#![cfg(feature = "cursup")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "cursup")))]
use crate::theme::Style;
use crate::utils::markup::{StyledIndexedSpan, StyledString};
use crate::utils::span::IndexedCow;

use unicode_width::UnicodeWidthStr;

/// Parse spans for the given text.
pub fn parse_spans(input: &str) -> Vec<StyledIndexedSpan> {
    let mut result = Vec::new();

    let re = regex::Regex::new(r"\/(?:(\w+)(?:\+(\w+))*)\{(.+)\}").unwrap();

    let mut offset = 0;
    let mut content = input;
    while let Some(c) = re.captures(content) {
        let m = c.get(0).unwrap();
        let start = m.start();
        let end = m.end();

        // First, append the entire content up to here.
        if start != 0 {
            result.push(StyledIndexedSpan {
                content: IndexedCow::Borrowed {
                    start: offset,
                    end: offset + start,
                },
                attr: Style::default(),
                width: content[..start].width(),
            });
        }

        let len = c.len();
        assert!(
            len > 2,
            "The regex should always yield at least 2 groups (+ the entire match)."
        );
        let body = c.get(len - 1).unwrap();

        let mut style = Style::default();

        for i in 1..len - 1 {
            let Some(action) = c.get(i) else {
                continue;
            };

            style = style.combine(action.as_str().parse::<Style>().unwrap());
        }

        result.push(StyledIndexedSpan {
            content: IndexedCow::Borrowed {
                start: offset + body.start(),
                end: offset + body.end(),
            },
            attr: style,
            width: body.as_str().width(),
        });

        content = &content[end..];
        offset += end;
    }

    if !content.is_empty() {
        result.push(StyledIndexedSpan {
            content: IndexedCow::Borrowed {
                start: offset,
                end: offset + content.len(),
            },
            attr: Style::default(),
            width: content.width(),
        });
    }

    result
}

/// Parse the given text into a styled string.
pub fn parse<S>(input: S) -> crate::utils::markup::StyledString
where
    S: Into<String>,
{
    let input = input.into();

    let spans = parse_spans(&input);

    StyledString::with_spans(input, spans)
}
