//! Parse various text markup formats.
//!
//! Each module is optional and relies on a feature.

#[cfg(feature = "markdown")]
pub mod markdown;

use crate::theme::Style;
use crate::utils::span::{IndexedSpan, Span, SpannedString};

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

/// Indexes a span into a source string.
pub type StyledIndexedSpan = IndexedSpan<Style>;

/// A resolved styled span borrowing its source string.
pub type StyledSpan<'a> = Span<'a, Style>;

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
