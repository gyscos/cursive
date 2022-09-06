//! Parse text with ANSI color codes.
//!
//! Needs the `ansi` feature to be enabled.
#![cfg(feature = "ansi")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "ansi")))]

use crate::theme::{BaseColor, Effect, Style};
use crate::utils::markup::{StyledIndexedSpan, StyledString};
use crate::utils::span::IndexedCow;

use ansi_parser::AnsiParser;
use unicode_width::UnicodeWidthStr;

/// Parses the given text with ANSI codes.
pub fn parse<S>(input: S) -> StyledString
where
    S: Into<String>,
{
    let input = input.into();

    let spans = parse_spans(&input);

    StyledString::with_spans(input, spans)
}

/// Parse the given text with ANSI codes into a list of spans.
///
/// This is a shortcut for `Parser::new(input).collect()`.
pub fn parse_spans(input: &str) -> Vec<StyledIndexedSpan> {
    Parser::new(input).collect()
}

/// Parses the given string as text with ANSI color codes.
pub struct Parser<'a> {
    input: &'a str,
    current_style: Style,
    parser: ansi_parser::AnsiParseIterator<'a>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser with the given input text.
    pub fn new(input: &'a str) -> Self {
        Parser {
            input,
            current_style: Style::default(),
            parser: input.ansi_parse(),
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = StyledIndexedSpan;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.parser.next()?;
            match next {
                ansi_parser::Output::TextBlock(text) => {
                    let width = text.width();
                    return Some(StyledIndexedSpan {
                        content: IndexedCow::from_str(text, self.input),
                        attr: self.current_style,
                        width,
                    });
                }
                ansi_parser::Output::Escape(sequence) => {
                    match sequence {
                        ansi_parser::AnsiSequence::SetGraphicsMode(bytes) => {
                            for byte in bytes {
                                match byte {
                                    0 => self.current_style = Style::default(),
                                    1 => {
                                        self.current_style
                                            .effects
                                            .insert(Effect::Bold);
                                        self.current_style
                                            .effects
                                            .remove(Effect::Dim);
                                    }
                                    2 => {
                                        self.current_style
                                            .effects
                                            .insert(Effect::Dim);
                                        self.current_style
                                            .effects
                                            .remove(Effect::Bold);
                                    }
                                    22 => {
                                        self.current_style
                                            .effects
                                            .remove(Effect::Dim);
                                        self.current_style
                                            .effects
                                            .remove(Effect::Bold);
                                    }
                                    3 => {
                                        self.current_style
                                            .effects
                                            .insert(Effect::Italic);
                                    }
                                    23 => {
                                        self.current_style
                                            .effects
                                            .remove(Effect::Italic);
                                    }
                                    4 => {
                                        self.current_style
                                            .effects
                                            .insert(Effect::Underline);
                                    }
                                    24 => {
                                        self.current_style
                                            .effects
                                            .remove(Effect::Underline);
                                    }
                                    5 | 6 => {
                                        // Technically 6 is rapid blink...
                                        self.current_style
                                            .effects
                                            .insert(Effect::Blink);
                                    }
                                    25 => {
                                        // Technically 6 is rapid blink...
                                        self.current_style
                                            .effects
                                            .remove(Effect::Blink);
                                    }
                                    7 => {
                                        self.current_style
                                            .effects
                                            .insert(Effect::Reverse);
                                    }
                                    27 => {
                                        self.current_style
                                            .effects
                                            .remove(Effect::Reverse);
                                    }
                                    9 => {
                                        self.current_style
                                            .effects
                                            .insert(Effect::Strikethrough);
                                    }
                                    29 => {
                                        self.current_style
                                            .effects
                                            .remove(Effect::Strikethrough);
                                    }
                                    30..=37 => {
                                        self.current_style.color.front =
                                            BaseColor::from(byte - 30)
                                                .dark()
                                                .into();
                                    }
                                    40..=47 => {
                                        self.current_style.color.back =
                                            BaseColor::from(byte - 40)
                                                .dark()
                                                .into();
                                    }
                                    90..=97 => {
                                        self.current_style.color.front =
                                            BaseColor::from(byte - 90)
                                                .light()
                                                .into();
                                    }
                                    100..=107 => {
                                        self.current_style.color.back =
                                            BaseColor::from(byte - 100)
                                                .light()
                                                .into();
                                    }
                                    _ => (),
                                }
                            }
                        }
                        // Nothing else to handle? Maybe SetMode/ResetMode?
                        _ => (),
                    }
                }
            }
        }
    }
}
