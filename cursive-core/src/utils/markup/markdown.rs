//! Parse markdown text.
//!
//! Needs the `markdown` feature to be enabled.
//!
//! ### Examples
//!
//! ```rust
//! use cursive::utils::markup::markdown::parse;
//! use cursive::views::TextView;
//! use cursive::{Cursive, CursiveExt};
//!
//! let mut siv = Cursive::default();
//!
//! let content = parse(
//!     r#"# Example Markdown
//! Hello, *world*!
//!
//! ## Header 2
//!
//! * List item 1
//! * List item 2
//! "#,
//! );
//!
//! siv.add_layer(TextView::new(content));
//! siv.run();
//! ```
#![cfg(feature = "markdown")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "markdown")))]

use std::borrow::Cow;

use crate::style::{Effect, Style};
use crate::utils::markup::{StyledIndexedSpan, StyledString};
use crate::utils::span::IndexedCow;

use pulldown_cmark::{self, CowStr, Event, LinkType, Tag, TagEnd};
use unicode_width::UnicodeWidthStr;

/// Parses the given string as Markdown text.
///
/// # Arguments
/// * `input` - The markdown text to parse.
///
/// # Returns
/// A `StyledString` containing the parsed text.
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

/// Iterator that parses a Markdown text and outputs styled spans.
pub struct Parser<'a> {
    /// Did we process the first item already?
    ///
    /// Used to omit newlines before the very first block.
    first: bool,
    stack: Vec<Style>,
    input: &'a str,
    parser: pulldown_cmark::Parser<'a>,
    /// In a code block, we keep newlines.
    in_codeblock: bool,
    /// Just added a new paragrap (\n\n)
    new_paragraph: bool,
}

impl<'a> Parser<'a> {
    /// Creates a new parser with the given input text.
    pub fn new(input: &'a str) -> Self {
        Parser {
            input,
            first: true,
            parser: pulldown_cmark::Parser::new(input),
            stack: Vec::new(),
            in_codeblock: false,
            new_paragraph: false,
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

impl Iterator for Parser<'_> {
    type Item = StyledIndexedSpan;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.parser.next()?;
            match next {
                Event::Start(tag) => match tag {
                    // Add to the stack!
                    Tag::Emphasis => self.stack.push(Style::from(Effect::Italic)),
                    Tag::Heading { level, .. } => {
                        /* Force a new paragraph before headings, but only if this is not
                         * the first output, and we have not skipped lines already.
                         */
                        let lines = if self.first {
                            ""
                        } else if !self.new_paragraph {
                            self.new_paragraph = true;
                            "\n\n"
                        } else {
                            self.new_paragraph = false;
                            ""
                        };
                        /* Headings are bold by default */
                        self.stack.push(Style::from(Effect::Bold));
                        return Some(self.literal(format!(
                            "{}{} ",
                            lines,
                            heading(level as usize)
                        )));
                    }
                    Tag::BlockQuote(_) => {
                        /* Push italic style */
                        self.stack.push(Style::from(Effect::Italic));
                        /* Force a blank line before a blockquote */
                        self.new_paragraph = true;
                        return Some(self.literal("\n\n> "));
                    }
                    Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        ..
                    } => {
                        self.new_paragraph = false;
                        return match link_type {
                            LinkType::Inline => {
                                /* There seems to be a bug in pulldown_cmark where it doesn't
                                 * recognize the link type. If the title is empty, we will
                                 * render an Autlink instead of an Inline link.
                                 */
                                return if title.is_empty() {
                                    Some(self.literal(format!("<{dest_url}> ")))
                                } else {
                                    Some(self.literal(format!("[{title}]({dest_url})")))
                                };
                            }
                            LinkType::Reference => {
                                Some(self.literal(format!("[{title}][{dest_url}]")))
                            }
                            LinkType::Collapsed => Some(self.literal(format!("[{title}][])"))),
                            LinkType::Shortcut => Some(self.literal(format!("[{title}]"))),
                            LinkType::Autolink => Some(self.literal(format!("<{dest_url}>"))),
                            _ => Some(self.literal(" ".to_string())),
                        };
                    }
                    Tag::CodeBlock(_) => {
                        self.in_codeblock = true;
                        if !self.new_paragraph {
                            self.new_paragraph = true;
                            return Some(self.literal("\n\n"));
                        }
                    }
                    Tag::Strong => self.stack.push(Style::from(Effect::Bold)),
                    Tag::Paragraph if !self.first => {
                        /* Do not add more than one paragraph */
                        if !self.new_paragraph {
                            self.new_paragraph = true;
                            return Some(self.literal("\n\n"));
                        }
                    }
                    _ => (),
                },
                Event::End(tag) => match tag {
                    // Remove from stack!
                    TagEnd::Paragraph if self.first => self.first = false,
                    TagEnd::Heading(..) => {
                        self.first = false;
                        /* Pop bold from style stack */
                        self.stack.pop().unwrap();
                        if !self.in_codeblock {
                            // After a heading we force a new paragraph.
                            self.new_paragraph = true;
                            return Some(self.literal("\n\n"));
                        }
                    }
                    TagEnd::CodeBlock => {
                        self.first = false;
                        self.in_codeblock = false;
                        // MT return Some(self.literal("```"));
                        self.new_paragraph = true;
                        return Some(self.literal("\n\n"));
                    }
                    TagEnd::BlockQuote(_) => {
                        /* Pop italic style */
                        self.stack.pop().unwrap();
                        self.new_paragraph = true;
                        return Some(self.literal("\n\n"));
                    }
                    TagEnd::Emphasis | TagEnd::Strong => {
                        self.stack.pop().unwrap();
                    }
                    _ => (),
                },
                Event::Rule => return Some(self.literal("---")),
                Event::SoftBreak => return Some(self.literal(" ")),
                Event::HardBreak => return Some(self.literal(" ")),
                // Treat all text the same
                Event::FootnoteReference(text)
                | Event::InlineHtml(text)
                | Event::Html(text)
                | Event::Text(text)
                | Event::Code(text)
                | Event::InlineMath(text)
                | Event::DisplayMath(text) => {
                    self.new_paragraph = false;
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
                    self.new_paragraph = false;
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
        let input = r#"
Attention
====
I *really* love __Cursive__!

> This is a blockquote

```rust
fn main() {
    println!("Goodbye, world!");
}
```

[Link](https://en.wikipedia.org)
"#;
        let parsed_spans = parse_spans(input);
        let parsed_spans: Vec<_> = parsed_spans
            .iter()
            .map(|span| span.resolve(input))
            .collect();

        // println!("{:?}", spans);
        let style_none = Style::none();
        let style_bold = Style::from(Effect::Bold);
        let style_italic = Style::from(Effect::Italic);

        let verify_spans = vec![
            Span {
                content: "# ",
                width: 2,
                attr: &style_bold,
            },
            Span {
                content: "Attention",
                width: 9,
                attr: &style_bold,
            },
            Span {
                content: "\n\n",
                width: "\n\n".width(),
                attr: &style_none,
            },
            Span {
                content: "I ",
                width: 2,
                attr: &style_none,
            },
            Span {
                content: "really",
                width: 6,
                attr: &style_italic,
            },
            Span {
                content: " love ",
                width: 6,
                attr: &style_none,
            },
            Span {
                content: "Cursive",
                width: 7,
                attr: &style_bold,
            },
            Span {
                content: "!",
                width: 1,
                attr: &style_none,
            },
            Span {
                content: "\n\n> ",
                width: "\n\n> ".width(),
                attr: &style_none,
            },
            Span {
                content: "This is a blockquote",
                width: 20,
                attr: &style_none,
            },
            Span {
                content: "\n\n",
                width: "\n\n".width(),
                attr: &style_none,
            },
            Span {
                content: "fn main() {\n    println!(\"Goodbye, world!\");\n}\n",
                width: "fn main() {\n    println!(\"Goodbye, world!\");\n}\n".width(),
                attr: &style_none,
            },
            Span {
                content: "\n\n",
                width: "\n\n".width(),
                attr: &style_none,
            },
            Span {
                content: "<https://en.wikipedia.org> ",
                width: "<https://en.wikipedia.org> ".width(),
                attr: &style_none,
            },
            Span {
                content: "Link",
                width: "Link".width(),
                attr: &style_none,
            },
            Span {
                content: "\n\n",
                width: "\n\n".width(),
                attr: &style_none,
            },
            Span {
                content: "\n\n",
                width: "\n\n".width(),
                attr: &style_none,
            },
        ];

        for i in 0..parsed_spans.len() {
            let result_span = &parsed_spans[i];
            assert_eq!(
                result_span.content, verify_spans[i].content,
                "Span {} is not equal",
                i
            );
        }
    }
}
