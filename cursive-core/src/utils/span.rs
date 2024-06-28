//! Work with spans of text.
//!
//! This module defines various structs describing a span of text from a
//! larger string.
use std::borrow::Cow;
use std::iter::FromIterator;
use unicode_width::UnicodeWidthStr;

/// A string with associated spans.
///
/// Each span has an associated attribute `T`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpannedString<T> {
    source: String,
    spans: Vec<IndexedSpan<T>>,
}

/// The immutable, borrowed equivalent of `SpannedString`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SpannedStr<'a, T> {
    source: &'a str,
    spans: &'a [IndexedSpan<T>],
}

// What we don't have: (&str, Vec<IndexedSpan<T>>)
// To style an existing text.
// Maybe replace `String` in `SpannedString` with `<S>`?

/// Describes an object that appears like a `SpannedStr`.
pub trait SpannedText {
    /// Type of span returned by `SpannedText::spans()`.
    ///
    /// Most of the time it'll be `IndexedSpan`.
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
    pub const fn new(source: &'a str, spans: &'a [IndexedSpan<T>]) -> Self {
        SpannedStr { source, spans }
    }

    /// Creates a new empty `SpannedStr`.
    pub const fn empty() -> Self {
        Self::new("", &[])
    }

    /// Gives access to the parsed styled spans.
    pub fn spans<'b>(
        &'b self,
    ) -> impl DoubleEndedIterator<Item = Span<'a, T>> + ExactSizeIterator<Item = Span<'a, T>> + 'b
    where
        'a: 'b,
    {
        let source = self.source;
        self.spans.iter().map(move |span| span.resolve(source))
    }

    /// Returns a reference to the indexed spans.
    pub const fn spans_raw(&self) -> &'a [IndexedSpan<T>] {
        self.spans
    }

    /// Returns a reference to the source (non-parsed) string.
    pub const fn source(&self) -> &'a str {
        self.source
    }

    /// Returns `true` if `self` is empty.
    ///
    /// Can be caused by an empty source, or no span.
    pub const fn is_empty(&self) -> bool {
        self.source.is_empty() || self.spans.is_empty()
    }

    /// Returns the width taken by this string.
    ///
    /// This is the sum of the width of each span.
    pub fn width(&self) -> usize {
        self.spans().map(|s| s.width).sum()
    }

    /// Create a new `SpannedStr` by borrowing from a `SpannedText`.
    pub fn from_spanned_text<'b, S>(text: &'b S) -> Self
    where
        S: SpannedText<S = IndexedSpan<T>>,
        'b: 'a,
    {
        Self {
            source: text.source(),
            spans: text.spans(),
        }
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

    /// Concatenates all styled strings.
    ///
    /// Same as `spans.into_iter().collect()`.
    pub fn concatenate<I>(spans: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        spans.into_iter().collect()
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
        // This should disappear when compiled in release mode.
        for span in &spans {
            if let IndexedCow::Borrowed { end, .. } = span.content {
                assert!(end <= source.len());
            }
        }

        SpannedString { source, spans }
    }

    /// Compacts and simplifies this string, resulting in a canonical form.
    ///
    /// If two styled strings represent the same styled text, they should have equal canonical
    /// forms.
    ///
    /// (The PartialEq implementation for StyledStrings requires both the source and spans to be
    /// equals, so non-visible changes such as text in the source between spans could cause
    /// StyledStrings to evaluate as non-equal.)
    pub fn canonicalize(&mut self)
    where
        T: PartialEq,
    {
        self.compact();
        self.simplify();
    }

    /// Returns the canonical form of this styled string.
    pub fn canonical(mut self) -> Self
    where
        T: PartialEq,
    {
        self.canonicalize();
        self
    }

    /// Compacts the source to only include the spans content.
    ///
    /// This does not change the number of spans, but changes the source.
    pub fn compact(&mut self) {
        // Prepare the new source
        let mut source = String::new();

        for span in &mut self.spans {
            // Only include what we need.
            let start = source.len();
            source.push_str(span.content.resolve(&self.source));
            let end = source.len();

            // All spans now borrow the source.
            span.content = IndexedCow::Borrowed { start, end };
        }

        self.source = source;
    }

    /// Attemps to reduce the number of spans by merging consecutive similar ones.
    pub fn simplify(&mut self)
    where
        T: PartialEq,
    {
        // Now, merge consecutive similar spans.
        let mut i = 0;
        while i + 1 < self.spans.len() {
            let left = &self.spans[i];
            let right = &self.spans[i + 1];
            if left.attr != right.attr {
                i += 1;
                continue;
            }

            let (_, left_end) = left.content.as_borrowed().unwrap();
            let (right_start, right_end) = right.content.as_borrowed().unwrap();
            let right_width = right.width;

            if left_end != right_start {
                i += 1;
                continue;
            }

            *self.spans[i].content.as_borrowed_mut().unwrap().1 = right_end;
            self.spans[i].width += right_width;
            self.spans.remove(i + 1);
        }
    }

    /// Shrink the source to discard any unused suffix.
    pub fn trim_end(&mut self) {
        if let Some(max) = self
            .spans
            .iter()
            .filter_map(|s| s.content.as_borrowed())
            .map(|(_start, end)| end)
            .max()
        {
            self.source.truncate(max);
        }
    }

    /// Shrink the source to discard any unused prefix.
    pub fn trim_start(&mut self) {
        if let Some(min) = self
            .spans
            .iter()
            .filter_map(|s| s.content.as_borrowed())
            .map(|(start, _end)| start)
            .min()
        {
            self.source.drain(..min);
            for span in &mut self.spans {
                span.content.rev_offset(min);
            }
        }
    }

    /// Shrink the source to discard any unused prefix or suffix.
    pub fn trim(&mut self) {
        self.trim_end();
        self.trim_start();
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

    /// Remove the given range of spans from the styled string.
    ///
    /// You may want to follow this with either `compact()`,
    /// `trim_start()` or `trim_end()`.
    pub fn remove_spans<R>(&mut self, range: R)
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.spans.drain(range);
    }

    /// Iterates on the resolved spans.
    pub fn spans(
        &self,
    ) -> impl DoubleEndedIterator<Item = Span<'_, T>> + ExactSizeIterator<Item = Span<'_, T>> {
        let source = &self.source;
        self.spans.iter().map(move |span| span.resolve(source))
    }

    /// Iterates on the resolved spans, with mutable access to the attributes.
    pub fn spans_attr_mut(&mut self) -> impl Iterator<Item = SpanMut<'_, T>> {
        let source = &self.source;
        self.spans
            .iter_mut()
            .map(move |span| span.resolve_mut(source))
    }

    /// Returns a reference to the indexed spans.
    pub fn spans_raw(&self) -> &[IndexedSpan<T>] {
        &self.spans
    }

    /// Returns a mutable iterator on the spans of this string.
    ///
    /// This can be used to modify the style of each span.
    pub fn spans_raw_attr_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = IndexedSpanRefMut<'_, T>>
           + ExactSizeIterator<Item = IndexedSpanRefMut<'_, T>> {
        self.spans.iter_mut().map(IndexedSpan::as_ref_mut)
    }

    /// Returns a reference to the source string.
    ///
    /// This is the non-parsed string.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the source, consuming this `StyledString`.
    pub fn into_source(self) -> String {
        self.source
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

impl<T> FromIterator<SpannedString<T>> for SpannedString<T> {
    fn from_iter<I: IntoIterator<Item = SpannedString<T>>>(iter: I) -> SpannedString<T> {
        // It would look simpler to always fold(), starting with an empty string.
        // But this here lets us re-use the allocation from the first string, which is a small win.
        let mut iter = iter.into_iter();
        if let Some(first) = iter.next() {
            iter.fold(first, |mut acc, s| {
                acc.append(s);
                acc
            })
        } else {
            SpannedString::new()
        }
    }
}

impl<'a, T> From<&'a SpannedString<T>> for SpannedStr<'a, T> {
    fn from(other: &'a SpannedString<T>) -> Self {
        SpannedStr::new(&other.source, &other.spans)
    }
}

/// A reference to an IndexedSpan allowing modification of the attribute.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct IndexedSpanRefMut<'a, T> {
    /// Points to the content of the span.
    pub content: &'a IndexedCow,

    /// Mutable reference to the attribute of the span.
    pub attr: &'a mut T,

    /// Width of the span.
    pub width: usize,
}

/// An indexed span with an associated attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// A resolved span borrowing its source string, with mutable access to the
/// attribute.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SpanMut<'a, T> {
    /// Content of this span.
    pub content: &'a str,

    /// Attribute associated to this span.
    pub attr: &'a mut T,

    /// Width of the text for this span.
    pub width: usize,
}

/// A resolved span borrowing its source string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Resolve the span to a string slice and a mutable attribute.
    pub fn resolve_mut<'a>(&'a mut self, source: &'a str) -> SpanMut<'a, T>
    where
        T: 'a,
    {
        SpanMut {
            content: self.content.resolve(source),
            attr: &mut self.attr,
            width: self.width,
        }
    }

    /// Returns a reference struct to only access mutation of the attribute.
    pub fn as_ref_mut(&mut self) -> IndexedSpanRefMut<'_, T> {
        IndexedSpanRefMut {
            content: &self.content,
            attr: &mut self.attr,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Gets a new `IndexedCow` for the given range.
    ///
    /// The given range is relative to this span.
    pub fn subcow(&self, range: std::ops::Range<usize>) -> Self {
        match *self {
            IndexedCow::Borrowed { start, end } => {
                if start + range.end > end {
                    panic!("Attempting to get a subcow larger than itself!");
                }
                IndexedCow::Borrowed {
                    start: start + range.start,
                    end: start + range.end,
                }
            }
            IndexedCow::Owned(ref content) => IndexedCow::Owned(content[range].into()),
        }
    }

    /// Return the `(start, end)` indexes if `self` is `IndexedCow::Borrowed`.
    pub fn as_borrowed(&self) -> Option<(usize, usize)> {
        if let IndexedCow::Borrowed { start, end } = *self {
            Some((start, end))
        } else {
            None
        }
    }

    /// Return the `(start, end)` indexes if `self` is `IndexedCow::Borrowed`.
    pub fn as_borrowed_mut(&mut self) -> Option<(&mut usize, &mut usize)> {
        if let IndexedCow::Borrowed {
            ref mut start,
            ref mut end,
        } = *self
        {
            Some((start, end))
        } else {
            None
        }
    }

    /// Returns the embedded text content if `self` is `IndexedCow::Owned`.
    pub fn as_owned(&self) -> Option<&str> {
        if let IndexedCow::Owned(ref content) = *self {
            Some(content)
        } else {
            None
        }
    }

    /// Returns an indexed view of the given string.
    ///
    /// **Note**: it is assumed `cow`, if borrowed, is a substring of `source`.
    pub fn from_str(value: &str, source: &str) -> Self {
        let source_pos = source.as_ptr() as usize;
        let value_pos = value.as_ptr() as usize;

        // Make sure `value` is indeed a substring of `source`
        assert!(value_pos >= source_pos);
        assert!(value_pos + value.len() <= source_pos + source.len());

        let start = value_pos - source_pos;
        let end = start + value.len();

        IndexedCow::Borrowed { start, end }
    }

    /// Returns an indexed view of the given item.
    ///
    /// **Note**: it is assumed `cow`, if borrowed, is a substring of `source`.
    pub fn from_cow(cow: Cow<'_, str>, source: &str) -> Self {
        match cow {
            Cow::Owned(value) => IndexedCow::Owned(value),
            Cow::Borrowed(value) => IndexedCow::from_str(value, source),
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
    /// Useful to update spans when concatenating sources. This span will now
    /// point to text `offset` further in the source.
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

    /// If `self` is borrowed, offset its indices back by the given value.
    ///
    /// Useful to update spans when removing a prefix from the source.
    /// This span will now point to text `offset` closer to the start of the source.
    ///
    /// This span may become empty as a result.
    pub fn rev_offset(&mut self, offset: usize) {
        if let IndexedCow::Borrowed {
            ref mut start,
            ref mut end,
        } = *self
        {
            *start = start.saturating_sub(offset);
            *end = end.saturating_sub(offset);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Style;

    #[test]
    fn test_spanned_str_width() {
        let spans = vec![
            IndexedSpan {
                content: IndexedCow::Borrowed { start: 0, end: 5 },
                attr: Style::default(),
                width: 5,
            },
            IndexedSpan {
                content: IndexedCow::Borrowed { start: 6, end: 11 },
                attr: Style::default(),
                width: 5,
            },
        ];
        let spanned_str = SpannedStr::new("Hello World", &spans);
        assert_eq!(spanned_str.width(), 10);
    }
}
