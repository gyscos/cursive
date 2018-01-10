//! Parse markdown text.
//!
//! Needs the `markdown` feature to be enabled.

extern crate pulldown_cmark;

use self::pulldown_cmark::{Event, Tag};
use std::borrow::Cow;
use theme::{Effect, Style};
use utils::lines::spans::Span;

/// Iterator that parse a markdown text and outputs styled spans.
pub struct Parser<'a> {
    first: bool,
    stack: Vec<Style>,
    parser: pulldown_cmark::Parser<'a>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser with the given input text.
    pub fn new(input: &'a str) -> Self {
        Parser {
            first: true,
            parser: pulldown_cmark::Parser::new(input),
            stack: Vec::new(),
        }
    }

    fn literal_string<'b>(&self, text: String) -> Span<'b> {
        Span {
            text: Cow::Owned(text),
            style: Style::merge(&self.stack),
        }
    }

    fn literal<'b>(&self, text: &'b str) -> Span<'b> {
        Span {
            text: Cow::Borrowed(text),
            style: Style::merge(&self.stack),
        }
    }
}

fn header(level: usize) -> &'static str {
    &"##########"[..level]
}

impl<'a> Iterator for Parser<'a> {
    type Item = Span<'a>;

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
                        return Some(self.literal_string(format!(
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
                        return Some(self.literal_string(format!(
                            "]({})",
                            link
                        )))
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
                    return Some(Span {
                        text,
                        style: Style::merge(&self.stack),
                    });
                }
            }
        }
    }
}

/// Parse the given markdown text into a list of spans.
///
/// This is a shortcut for `Parser::new(input).collect()`.
pub fn parse<'a>(input: &'a str) -> Vec<Span<'a>> {
    Parser::new(input).collect()
}

/// `Markup` trait implementation for markdown text.
///
/// Requires the `markdown` feature.
pub struct Markdown;

impl super::Markup for Markdown {
    type Error = ();

    fn parse<'a>(input: &'a str) -> Result<Vec<Span<'a>>, Self::Error> {
        Ok(parse(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let input = r"
Attention
====
I *really* love __Cursive__!";
        let spans = parse(input);

        // println!("{:?}", spans);
        assert_eq!(
            &spans[..],
            &[
                Span {
                    text: Cow::Borrowed("# "),
                    style: Style::none(),
                },
                Span {
                    text: Cow::Borrowed("Attention"),
                    style: Style::none(),
                },
                Span {
                    text: Cow::Borrowed("\n\n"),
                    style: Style::none(),
                },
                Span {
                    text: Cow::Borrowed("I "),
                    style: Style::none(),
                },
                Span {
                    text: Cow::Borrowed("really"),
                    style: Style::from(Effect::Italic),
                },
                Span {
                    text: Cow::Borrowed(" love "),
                    style: Style::none(),
                },
                Span {
                    text: Cow::Borrowed("Cursive"),
                    style: Style::from(Effect::Bold),
                },
                Span {
                    text: Cow::Borrowed("!"),
                    style: Style::none(),
                }
            ]
        );
    }
}
