//! Parse text with ANSI color codes.
//!
//! Needs the `ansi` feature to be enabled.
#![cfg(feature = "ansi")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "ansi")))]

use crate::style::{BaseColor, Color, Effect, Style};
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

    let spans = Parser::new(&input).collect();

    StyledString::with_spans(input, spans)
}

/// Parses the given text with ANSI codes, using the given starting style.
///
/// Useful if you need to parse something in the middle of a large text.
///
/// Returns the parsed string, and the ending style.
pub fn parse_with_starting_style<S>(current_style: Style, input: S) -> (StyledString, Style)
where
    S: Into<String>,
{
    let input = input.into();

    let mut parser = Parser::with_starting_style(current_style, &input);
    let spans = (&mut parser).collect();
    let ending_style = parser.current_style();

    (StyledString::with_spans(input, spans), ending_style)
}

/// Parses the given string as text with ANSI color codes.
pub struct Parser<'a> {
    input: &'a str,
    current_style: Style,
    parser: ansi_parser::AnsiParseIterator<'a>,
}

fn parse_color(mut bytes: impl Iterator<Item = u8>) -> Option<Color> {
    Some(match bytes.next()? {
        5 => {
            let color = bytes.next()?;
            Color::from_256colors(color)
        }
        2 => {
            let r = bytes.next()?;
            let g = bytes.next()?;
            let b = bytes.next()?;
            Color::Rgb(r, g, b)
        }
        _ => {
            // ???
            return None;
        }
    })
}

impl<'a> Parser<'a> {
    /// Creates a new parser with the given input text.
    pub fn new(input: &'a str) -> Self {
        Self::with_starting_style(Style::default(), input)
    }

    /// Returns the current style.
    pub fn current_style(&self) -> Style {
        self.current_style
    }

    /// Creates a new parser with the given input text,
    /// using the given initial style.
    ///
    /// Useful if you need to parse something in the middle of a large text.
    pub fn with_starting_style(current_style: Style, input: &'a str) -> Self {
        Parser {
            input,
            current_style,
            parser: input.ansi_parse(),
        }
    }

    fn parse_sequence(&mut self, seq: &[u8]) -> Option<()> {
        let mut bytes = seq.iter().copied();
        loop {
            let byte = bytes.next()?;

            match byte {
                0 => self.current_style = Style::default(),
                1 => {
                    self.current_style.effects.insert(Effect::Bold);
                    self.current_style.effects.remove(Effect::Dim);
                }
                2 => {
                    self.current_style.effects.insert(Effect::Dim);
                    self.current_style.effects.remove(Effect::Bold);
                }
                22 => {
                    self.current_style.effects.remove(Effect::Dim);
                    self.current_style.effects.remove(Effect::Bold);
                }
                3 => {
                    self.current_style.effects.insert(Effect::Italic);
                }
                23 => {
                    self.current_style.effects.remove(Effect::Italic);
                }
                4 => {
                    self.current_style.effects.insert(Effect::Underline);
                }
                24 => {
                    self.current_style.effects.remove(Effect::Underline);
                }
                5 | 6 => {
                    // Technically 6 is rapid blink...
                    self.current_style.effects.insert(Effect::Blink);
                }
                25 => {
                    // Technically 6 is rapid blink...
                    self.current_style.effects.remove(Effect::Blink);
                }
                7 => {
                    self.current_style.effects.insert(Effect::Reverse);
                }
                27 => {
                    self.current_style.effects.remove(Effect::Reverse);
                }
                9 => {
                    self.current_style.effects.insert(Effect::Strikethrough);
                }
                29 => {
                    self.current_style.effects.remove(Effect::Strikethrough);
                }
                30..=37 => {
                    self.current_style.color.front = BaseColor::from(byte - 30).dark().into();
                }
                38 => {
                    self.current_style.color.front = parse_color(&mut bytes)?.into();
                }
                39 => {
                    self.current_style.color.front = Color::TerminalDefault.into();
                }
                40..=47 => {
                    self.current_style.color.back = BaseColor::from(byte - 40).dark().into();
                }
                48 => {
                    self.current_style.color.back = parse_color(&mut bytes)?.into();
                }
                49 => {
                    self.current_style.color.back = Color::TerminalDefault.into();
                }
                58 => {
                    // Set underline color.
                    // Not implemented, but consumes the rest
                    parse_color(&mut bytes)?;
                }
                90..=97 => {
                    self.current_style.color.front = BaseColor::from(byte - 90).light().into();
                }
                100..=107 => {
                    self.current_style.color.back = BaseColor::from(byte - 100).light().into();
                }
                _ => (),
            }
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
                    if let ansi_parser::AnsiSequence::SetGraphicsMode(bytes) = sequence {
                        self.parse_sequence(&bytes);
                    }
                    // Nothing else to handle? Maybe SetMode/ResetMode?
                }
            }
        }
    }
}
