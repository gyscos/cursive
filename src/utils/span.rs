//! Work with spans of text.
//!
//! This module defines various structs describing a span of text from a
//! larger string.
use std::borrow::Cow;
use unicode_width::UnicodeWidthStr;

/// A string with associated spans.
///
/// Each span has an associated attribute `T`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedString<T> {
    source: String,
    spans: Vec<IndexedSpan<T>>,
}

/// The immutable, borrowed equivalent of `SpannedString`.
#[derive(Debug, PartialEq, Eq)]
pub struct SpannedStr<'a, T> {
    source: &'a str,
    spans: &'a [IndexedSpan<T>],
}

/// Describes an object that appears like a `SpannedStr`.
pub trait SpannedText {
    /// Type of span returned by `SpannedText::spans()`.
    type S: AsRef<IndexedCow>;

    /// Returns the source text.
    fn source(&self) -> &str;

    /// Returns the spans for this text.
    fn spans(&self) -> &[Self::S];

    /// Returns a `SpannedText` by reference.
    fn as_ref(&self) -> SpannedTextRef<'_, Self> {
        SpannedTextRef { r: self }
    }
}

/// A reference to another `SpannedText`.
pub struct SpannedTextRef<'a, C>
where
    C: SpannedText + ?Sized,
{
    r: &'a C,
}

impl<T> Default for SpannedString<T> {
    fn default() -> Self {
        SpannedString::new()
    }
}

impl<'a, T> SpannedText for &'a SpannedString<T> {
    type S = IndexedSpan<T>;

    fn source(&self) -> &str {
        &self.source
    }

    fn spans(&self) -> &[IndexedSpan<T>] {
        &self.spans
    }
}

impl<'a, C> SpannedText for SpannedTextRef<'a, C>
where
    C: 'a + SpannedText + ?Sized,
{
    type S = C::S;

    fn source(&self) -> &str {
        self.r.source()
    }

    fn spans(&self) -> &[C::S] {
        self.r.spans()
    }
}

impl<'a, T> SpannedText for SpannedStr<'a, T>
where
    T: 'a,
{
    type S = IndexedSpan<T>;

    fn source(&self) -> &str {
        self.source
    }

    fn spans(&self) -> &[IndexedSpan<T>] {
        self.spans
    }
}

impl<S, T> From<S> for SpannedString<T>
where
    S: Into<String>,
    T: Default,
{
    fn from(value: S) -> Self {
        Self::single_span(value.into(), T::default())
    }
}

impl<'a, T> SpannedStr<'a, T>
where
    T: 'a,
{
    /// Creates a new `SpannedStr` from the given references.
    pub fn new(source: &'a str, spans: &'a [IndexedSpan<T>]) -> Self {
        SpannedStr { source, spans }
    }

    /// Gives access to the parsed styled spans.
    pub fn spans<'b>(&'b self) -> impl Iterator<Item = Span<'a, T>> + 'b
    where
        'a: 'b,
    {
        let source = self.source;
        self.spans.iter().map(move |span| span.resolve(source))
    }

    /// Returns a reference to the indexed spans.
    pub fn spans_raw(&self) -> &'a [IndexedSpan<T>] {
        self.spans
    }

    /// Returns a reference to the source (non-parsed) string.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Returns `true` if `self` is empty.
    ///
    /// Can be caused by an empty source, or no span.
    pub fn is_empty(&self) -> bool {
        self.source.is_empty() || self.spans.is_empty()
    }
}

impl<'a, T> Clone for SpannedStr<'a, T> {
    fn clone(&self) -> Self {
        SpannedStr {
            source: self.source,
            spans: self.spans,
        }
    }
}

impl SpannedString<()> {
    /// Returns a simple spanned string without any attribute.
    pub fn plain<S>(content: S) -> Self
    where
        S: Into<String>,
    {
        Self::single_span(content, ())
    }
}

impl<T> SpannedString<T> {
    /// Returns an empty `SpannedString`.
    pub fn new() -> Self {
        Self::with_spans(String::new(), Vec::new())
    }

    /// Creates a new `SpannedString` manually.
    ///
    /// It is not recommended to use this directly.
    /// Instead, look for methods like `Markdown::parse`.
    pub fn with_spans<S>(source: S, spans: Vec<IndexedSpan<T>>) -> Self
    where
        S: Into<String>,
    {
        let source = source.into();

        // Make sure the spans are within bounds.
        // This should disapear when compiled in release mode.
        for span in &spans {
            if let IndexedCow::Borrowed { end, .. } = span.content {
                assert!(end <= source.len());
            }
        }

        SpannedString { source, spans }
    }

    /// Returns a new SpannedString with a single span.
    pub fn single_span<S>(source: S, attr: T) -> Self
    where
        S: Into<String>,
    {
        let source = source.into();

        let spans = vec![IndexedSpan::simple_borrowed(&source, attr)];

        Self::with_spans(source, spans)
    }

    /// Appends the given `StyledString` to `self`.
    pub fn append<S>(&mut self, other: S)
    where
        S: Into<Self>,
    {
        let other = other.into();
        self.append_raw(&other.source, other.spans);
    }

    /// Appends `content` and its corresponding spans to the end.
    ///
    /// It is not recommended to use this directly;
    /// instead, look at the `append` method.
    pub fn append_raw(&mut self, source: &str, spans: Vec<IndexedSpan<T>>) {
        let offset = self.source.len();
        let mut spans = spans;

        for span in &mut spans {
            span.content.offset(offset);
        }

        self.source.push_str(source);
        self.spans.append(&mut spans);
    }

    /// Gives access to the parsed styled spans.
    pub fn spans(&self) -> impl Iterator<Item = Span<'_, T>> {
        let source = &self.source;
        self.spans.iter().map(move |span| span.resolve(source))
    }

    /// Returns a reference to the indexed spans.
    pub fn spans_raw(&self) -> &[IndexedSpan<T>] {
        &self.spans
    }

    /// Returns a reference to the source string.
    ///
    /// This is the non-parsed string.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns `true` if self is empty.
    pub fn is_empty(&self) -> bool {
        self.source.is_empty() || self.spans.is_empty()
    }

    /// Returns the width taken by this string.
    ///
    /// This is the sum of the width of each span.
    pub fn width(&self) -> usize {
        self.spans().map(|s| s.width).sum()
    }
}

impl<'a, T> From<&'a SpannedString<T>> for SpannedStr<'a, T> {
    fn from(other: &'a SpannedString<T>) -> Self {
        SpannedStr::new(&other.source, &other.spans)
    }
}

/// An indexed span with an associated attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSpan<T> {
    /// Content of the span.
    pub content: IndexedCow,

    /// Attribute applied to the span.
    pub attr: T,

    /// Width of the text for this span.
    pub width: usize,
}

impl<T> AsRef<IndexedCow> for IndexedSpan<T> {
    fn as_ref(&self) -> &IndexedCow {
        &self.content
    }
}

/// A resolved span borrowing its source string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span<'a, T> {
    /// Content of this span.
    pub content: &'a str,

    /// Attribute associated to this span.
    pub attr: &'a T,

    /// Width of the text for this span.
    pub width: usize,
}

impl<T> IndexedSpan<T> {
    /// Resolve the span to a string slice and an attribute.
    pub fn resolve<'a>(&'a self, source: &'a str) -> Span<'a, T>
    where
        T: 'a,
    {
        Span {
            content: self.content.resolve(source),
            attr: &self.attr,
            width: self.width,
        }
    }

    /// Returns `true` if `self` is an empty span.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Returns a single indexed span around the entire text.
    pub fn simple_borrowed(content: &str, attr: T) -> Self {
        IndexedSpan {
            content: IndexedCow::Borrowed {
                start: 0,
                end: content.len(),
            },
            attr,
            width: content.width(),
        }
    }

    /// Returns a single owned indexed span around the entire text.
    pub fn simple_owned(content: String, attr: T) -> Self {
        let width = content.width();
        IndexedSpan {
            content: IndexedCow::Owned(content),
            attr,
            width,
        }
    }
}

/// A span of text that can be either owned, or indexed in another String.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexedCow {
    /// Indexes content in a separate string.
    Borrowed {
        /// Byte offset of the beginning of the span (inclusive)
        start: usize,

        /// Byte offset of the end of the span (exclusive)
        end: usize,
    },

    /// Owns its content.
    Owned(String),
}

impl IndexedCow {
    /// Resolve the span to a string slice.
    pub fn resolve<'a>(&'a self, source: &'a str) -> &'a str {
        match *self {
            IndexedCow::Borrowed { start, end } => &source[start..end],
            IndexedCow::Owned(ref content) => content,
        }
    }

    /// Returns an indexed view of the given item.
    ///
    /// **Note**: it is assumed `cow`, if borrowed, is a substring of `source`.
    pub fn from_cow(cow: Cow<'_, str>, source: &str) -> Self {
        match cow {
            Cow::Owned(value) => IndexedCow::Owned(value),
            Cow::Borrowed(value) => {
                let source_pos = source.as_ptr() as usize;
                let value_pos = value.as_ptr() as usize;

                // Make sure `value` is indeed a substring of `source`
                assert!(value_pos >= source_pos);
                assert!(value_pos + value.len() <= source_pos + source.len());
                let start = value_pos - source_pos;
                let end = start + value.len();

                IndexedCow::Borrowed { start, end }
            }
        }
    }

    /// Returns `true` if this represents an empty span.
    pub fn is_empty(&self) -> bool {
        match *self {
            IndexedCow::Borrowed { start, end } => start == end,
            IndexedCow::Owned(ref content) => content.is_empty(),
        }
    }

    /// If `self` is borrowed, offset its indices by the given value.
    ///
    /// Useful to update spans when concatenating sources.
    pub fn offset(&mut self, offset: usize) {
        if let IndexedCow::Borrowed {
            ref mut start,
            ref mut end,
        } = *self
        {
            *start += offset;
            *end += offset;
        }
    }
}
