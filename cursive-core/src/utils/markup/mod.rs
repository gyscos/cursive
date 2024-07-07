//! Parse various text markup formats.
//!
//! Each module is optional and relies on a feature.

pub mod ansi;
pub mod cursup;
pub mod gradient;
pub mod markdown;

use crate::style::Style;
use crate::utils::span::{IndexedCow, IndexedSpan, Span, SpannedStr, SpannedString, SpannedText};

use unicode_width::UnicodeWidthStr;

/// A parsed string with markup style.
///
/// Contains both the source string, and parsed information indicating the
/// style to apply.
///
/// **Note**: due to limitations in rustdoc, you will need to read the
/// documentation from the [`SpannedString`] page.
///
/// [`SpannedString`]: ../span/struct.SpannedString.html
pub type StyledString = SpannedString<Style>;

/// A borrowed parsed string with markup style.
pub type StyledStr<'a> = SpannedStr<'a, Style>;

/// Indexes a span into a source string.
pub type StyledIndexedSpan = IndexedSpan<Style>;

/// A resolved styled span borrowing its source string.
pub type StyledSpan<'a> = Span<'a, Style>;

/// Represents plain text without any style.
pub struct PlainStr<'a> {
    source: &'a str,
    span: IndexedSpan<Style>,
}

impl<'a> SpannedText for PlainStr<'a> {
    type S = IndexedSpan<Style>;

    fn source(&self) -> &str {
        self.source
    }

    fn spans(&self) -> &[Self::S] {
        std::slice::from_ref(&self.span)
    }
}

impl<'a> PlainStr<'a> {
    /// Create a new `PlainStr` with the given content.
    pub fn new(source: &'a str) -> Self {
        Self::new_with_width(source, source.width())
    }

    /// Create a new `PlainStr` with the given content.
    ///
    /// Relies on the given width being correct for the source.
    ///
    /// While it is not _strictly unsafe_ to give the incorrect width, it can
    /// lead to logic errors and incorrect output.
    ///
    /// This function mostly exists if:
    /// * You need to call it in a `const` context.
    /// * You already have the width of the source and do not want to waste compute cycles.
    ///
    /// In most cases you should be using `new` instead.
    pub const fn new_with_width(source: &'a str, width: usize) -> Self {
        // TODO: Rely on a `StrWithWidth` or something that embeds the width there.
        // Relies on proc-macro to const-generate from a literal.
        let span = IndexedSpan {
            content: IndexedCow::Borrowed {
                start: 0,
                end: source.len(),
            },
            attr: Style::none(),
            width,
        };
        Self { source, span }
    }

    /// Get a `StyledStr` borrowing this `PlainStr`.
    pub fn as_styled_str(&self) -> StyledStr {
        StyledStr::from_spanned_text(self)
    }
}

impl SpannedString<Style> {
    /// Returns a plain StyledString without any style.
    ///
    /// > You got no style, Dutch. You know that.
    pub fn plain<S>(content: S) -> Self
    where
        S: Into<String>,
    {
        Self::styled(content, Style::none())
    }

    /// Creates a new `StyledString` using a single style for the entire text.
    pub fn styled<S, T>(content: S, style: T) -> Self
    where
        S: Into<String>,
        T: Into<Style>,
    {
        let content = content.into();
        let style = style.into();

        Self::single_span(content, style)
    }

    /// Appends the given plain text to `self`.
    pub fn append_plain<S>(&mut self, text: S)
    where
        S: Into<String>,
    {
        self.append(Self::plain(text));
    }

    /// Appends `text` to `self`, using `style`.
    pub fn append_styled<S, T>(&mut self, text: S, style: T)
    where
        S: Into<String>,
        T: Into<Style>,
    {
        self.append(Self::styled(text, style));
    }
}
