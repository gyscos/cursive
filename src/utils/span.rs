//! Work with spans of text.
//!
//! This module defines various structs describing a span of text from a
//! larger string.
use std::borrow::Cow;

/// A string with associated spans.
///
/// Each span has an associated attribute `T`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedString<T> {
    source: String,
    spans: Vec<IndexedSpan<T>>,
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

impl<T> SpannedString<T> {
    /// Creates a new `SpannedString` manually.
    ///
    /// It is not recommended to use this directly.
    /// Instead, look for methods like `Markdown::parse`.
    pub fn new<S>(source: S, spans: Vec<IndexedSpan<T>>) -> Self
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
    pub fn single_span(source: String, attr: T) -> Self {
        let spans = vec![
            IndexedSpan {
                content: IndexedCow::Borrowed {
                    start: 0,
                    end: source.len(),
                },
                attr,
            },
        ];

        Self::new(source, spans)
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
    #[cfg_attr(feature = "cargo-clippy", allow(needless_lifetimes))]
    pub fn spans<'a>(&'a self) -> Vec<Span<'a, T>> {
        self.spans
            .iter()
            .map(|span| span.resolve(&self.source))
            .collect()
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
}

/// An indexed span with an associated attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSpan<T> {
    /// Content of the span.
    pub content: IndexedCow,

    /// Attribute applied to the span.
    pub attr: T,
}

/// A resolved span borrowing its source string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span<'a, T: 'a> {
    /// Content of this span.
    pub content: &'a str,

    /// Attribute associated to this span.
    pub attr: &'a T,
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
        }
    }

    /// Returns `true` if `self` is an empty span.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
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
    pub fn from_cow(cow: Cow<str>, source: &str) -> Self {
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

    /// Returns `ŧrue` if this represents an empty span.
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
