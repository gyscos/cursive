//! Parse markdown text.
//!
//! Needs the `markdown` feature to be enabled.
#![cfg(feature = "markdown")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "markdown")))]

use std::borrow::Cow;

use crate::style::{Effect, Style};
use crate::utils::markup::{StyledIndexedSpan, StyledString};
use crate::utils::span::IndexedCow;

use pulldown_cmark::{self, CowStr, Event, Tag, TagEnd};
use unicode_width::UnicodeWidthStr;

/// Parses the given string as markdown text.
pub fn parse<S>(input: S) -> StyledString
where
    S: Into<String>,
{
    let input = input.into();

    let spans = parse_spans(&input);

    StyledString::with_spans(input, spans)
}

// Convert a CowStr from pulldown into a regular Cow<str>
// We lose the inline optimization, but oh well.
fn cowvert(cow: CowStr) -> Cow<str> {
    match cow {
        CowStr::Borrowed(text) => Cow::Borrowed(text),
        CowStr::Boxed(text) => Cow::Owned(text.into()),
        CowStr::Inlined(text) => Cow::Owned(text.to_string()),
    }
}

/// Iterator that parse a markdown text and outputs styled spans.
pub struct Parser<'a> {
    first: bool,
    stack: Vec<Style>,
    input: &'a str,
    parser: pulldown_cmark::Parser<'a>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser with the given input text.
    pub fn new(input: &'a str) -> Self {
        Parser {
            input,
            first: true,
            parser: pulldown_cmark::Parser::new(input),
            stack: Vec::new(),
        }
    }

    /// Creates a new span with the given value
    fn literal<S>(&self, text: S) -> StyledIndexedSpan
    where
        S: Into<String>,
    {
        StyledIndexedSpan::simple_owned(text.into(), Style::merge(&self.stack))
    }
}

fn heading(level: usize) -> &'static str {
    &"##########"[..level]
}

impl<'a> Iterator for Parser<'a> {
    type Item = StyledIndexedSpan;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = match self.parser.next() {
                None => return None,
                Some(event) => event,
            };

            match next {
                Event::Start(tag) => match tag {
                    // Add to the stack!
                    Tag::Emphasis => self.stack.push(Style::from(Effect::Italic)),
                    Tag::Heading { level, .. } => {
                        return Some(self.literal(format!("{} ", heading(level as usize))))
                    }
                    Tag::BlockQuote(_) => return Some(self.literal("> ")),
                    Tag::Link {
                        dest_url, title, ..
                    } => return Some(self.literal(format!("[{title}]({dest_url})"))),
                    Tag::CodeBlock(_) => return Some(self.literal("```")),
                    Tag::Strong => self.stack.push(Style::from(Effect::Bold)),
                    Tag::Paragraph if !self.first => return Some(self.literal("\n\n")),
                    _ => (),
                },
                Event::End(tag) => match tag {
                    // Remove from stack!
                    TagEnd::Paragraph if self.first => self.first = false,
                    TagEnd::Heading(..) => return Some(self.literal("\n\n")),
                    TagEnd::CodeBlock => return Some(self.literal("```")),
                    TagEnd::Emphasis | TagEnd::Strong => {
                        self.stack.pop().unwrap();
                    }
                    _ => (),
                },
                Event::Rule => return Some(self.literal("---")),
                Event::SoftBreak => return Some(self.literal("\n")),
                Event::HardBreak => return Some(self.literal("\n")),
                // Treat all text the same
                Event::FootnoteReference(text)
                | Event::InlineHtml(text)
                | Event::Html(text)
                | Event::Text(text)
                | Event::Code(text)
                | Event::InlineMath(text)
                | Event::DisplayMath(text) => {
                    let text = cowvert(text);
                    let width = text.width();
                    // Return something!
                    return Some(StyledIndexedSpan {
                        content: IndexedCow::from_cow(text, self.input),
                        attr: Style::merge(&self.stack),
                        width,
                    });
                }
                Event::TaskListMarker(checked) => {
                    let mark = if checked { "[x]" } else { "[ ]" };
                    return Some(self.literal(mark));
                }
            }
        }
    }
}

/// Parse the given markdown text into a list of spans.
///
/// This is a shortcut for `Parser::new(input).collect()`.
pub fn parse_spans(input: &str) -> Vec<StyledIndexedSpan> {
    Parser::new(input).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::span::Span;

    #[test]
    fn test_parse() {
        let input = r"
Attention
====
I *really* love __Cursive__!";
        let spans = parse_spans(input);
        let spans: Vec<_> = spans.iter().map(|span| span.resolve(input)).collect();

        // println!("{:?}", spans);
        assert_eq!(
            &spans[..],
            &[
                Span {
                    content: "# ",
                    width: 2,
                    attr: &Style::none(),
                },
                Span {
                    content: "Attention",
                    width: 9,
                    attr: &Style::none(),
                },
                Span {
                    content: "\n\n",
                    width: "\n\n".width(),
                    attr: &Style::none(),
                },
                Span {
                    content: "I ",
                    width: 2,
                    attr: &Style::none(),
                },
                Span {
                    content: "really",
                    width: 6,
                    attr: &Style::from(Effect::Italic),
                },
                Span {
                    content: " love ",
                    width: 6,
                    attr: &Style::none(),
                },
                Span {
                    content: "Cursive",
                    width: 7,
                    attr: &Style::from(Effect::Bold),
                },
                Span {
                    content: "!",
                    width: 1,
                    attr: &Style::none(),
                }
            ]
        );
    }
}
