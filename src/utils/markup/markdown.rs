//! Parse markdown text.
//!
//! Needs the `markdown` feature to be enabled.

extern crate pulldown_cmark;

use self::pulldown_cmark::{Event, Tag};
use theme::{Effect, Style};
use utils::markup::{StyledIndexedSpan, StyledString};
use utils::span::IndexedCow;

/// Parses the given string as markdown text.
pub fn parse<S>(input: S) -> StyledString
where
    S: Into<String>,
{
    let input = input.into();

    let spans = parse_spans(&input);

    StyledString::new(input, spans)
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
        StyledIndexedSpan {
            content: IndexedCow::Owned(text.into()),
            attr: Style::merge(&self.stack),
        }
    }
}

fn header(level: usize) -> &'static str {
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
                    Tag::Emphasis => {
                        self.stack.push(Style::from(Effect::Italic))
                    }
                    Tag::Header(level) => {
                        return Some(self.literal(format!(
                            "{} ",
                            header(level as usize)
                        )))
                    }
                    Tag::Rule => return Some(self.literal("---")),
                    Tag::BlockQuote => return Some(self.literal("> ")),
                    Tag::Link(_, _) => return Some(self.literal("[")),
                    Tag::Code => return Some(self.literal("```")),
                    Tag::Strong => self.stack.push(Style::from(Effect::Bold)),
                    Tag::Paragraph if !self.first => {
                        return Some(self.literal("\n\n"))
                    }
                    _ => (),
                },
                Event::End(tag) => match tag {
                    // Remove from stack!
                    Tag::Paragraph if self.first => self.first = false,
                    Tag::Header(_) => return Some(self.literal("\n\n")),
                    Tag::Link(link, _) => {
                        return Some(self.literal(format!("]({})", link)))
                    }
                    Tag::Code => return Some(self.literal("```")),
                    Tag::Emphasis | Tag::Strong => {
                        self.stack.pop().unwrap();
                    }
                    _ => (),
                },
                Event::SoftBreak => return Some(self.literal("\n")),
                Event::HardBreak => return Some(self.literal("\n")),
                // Treat all text the same
                Event::FootnoteReference(text)
                | Event::InlineHtml(text)
                | Event::Html(text)
                | Event::Text(text) => {
                    // Return something!
                    return Some(StyledIndexedSpan {
                        content: IndexedCow::from_cow(text, self.input),
                        attr: Style::merge(&self.stack),
                    });
                }
            }
        }
    }
}

/// Parse the given markdown text into a list of spans.
///
/// This is a shortcut for `Parser::new(input).collect()`.
pub fn parse_spans<'a>(input: &'a str) -> Vec<StyledIndexedSpan> {
    Parser::new(input).collect()
    // Parser::new(input).inspect(|span| eprintln!("{:?}", span)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::span::Span;

    #[test]
    fn test_parse() {
        let input = r"
Attention
====
I *really* love __Cursive__!";
        let spans = parse_spans(input);
        let spans: Vec<_> = spans
            .iter()
            .map(|span| span.resolve(input))
            .collect();

        // println!("{:?}", spans);
        assert_eq!(
            &spans[..],
            &[
                Span {
                    content: "# ",
                    attr: &Style::none(),
                },
                Span {
                    content: "Attention",
                    attr: &Style::none(),
                },
                Span {
                    content: "\n\n",
                    attr: &Style::none(),
                },
                Span {
                    content: "I ",
                    attr: &Style::none(),
                },
                Span {
                    content: "really",
                    attr: &Style::from(Effect::Italic),
                },
                Span {
                    content: " love ",
                    attr: &Style::none(),
                },
                Span {
                    content: "Cursive",
                    attr: &Style::from(Effect::Bold),
                },
                Span {
                    content: "!",
                    attr: &Style::none(),
                }
            ]
        );
    }
}
