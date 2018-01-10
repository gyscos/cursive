//! Parse various text markup formats.
//!
//! Each module is optional and relies on a feature.

#[cfg(feature = "markdown")]
pub mod markdown;

#[cfg(feature = "markdown")]
pub use self::markdown::Markdown;
use owning_ref::OwningHandle;
use owning_ref::StringRef;
use std::marker::PhantomData;
use utils::lines::spans::Span;

/// Trait for parsing text into styled spans.
pub trait Markup {
    /// Possible error happening when parsing.
    type Error;

    /// Parses text and return the styled spans.
    fn parse<'a>(input: &'a str) -> Result<Vec<Span<'a>>, Self::Error>;

    /// Returns a string and its parsed spans.
    ///
    /// Generates a self-borrowing struct containing the source string, as well
    /// as the styled spans borrowing this string.
    fn make_handle<S>(input: S) -> Result<StyledHandle, Self::Error>
    where
        S: Into<String>,
    {
        let input = input.into();
        OwningHandle::try_new(StringRef::new(input), |input| {
            Self::parse(unsafe { &*input })
        })
    }
}

/// Holds both parsed spans, and the input string they borrow.
///
/// This is used to pass around a parsed string.
pub type StyledHandle = OwningHandle<StringRef, Vec<Span<'static>>>;

/// A String that parses a markup language.
pub struct StyledString {
    content: StyledHandle,
}

impl StyledString {

    /// Creates a new styled string, parsing the given content.
    pub fn new<S, M>(content: S) -> Result<Self, M::Error>
    where
        S: Into<String>,
        M: Markup,
    {
        let content = M::make_handle(content)?;
        Ok(StyledString { content })
    }

    /// Sets the content of this string.
    ///
    /// The content will be parsed; if an error is found,
    /// it will be returned here (and the content will be unchanged).
    pub fn set_content<S, M>(&mut self, content: S) -> Result<(), M::Error>
    where
        S: Into<String>,
        M: Markup,
    {
        self.content = M::make_handle(content)?;

        Ok(())
    }

    /// Gives access to the parsed styled spans.
    pub fn spans<'a>(&'a self) -> &'a [Span<'a>] {
        &self.content
    }
}
