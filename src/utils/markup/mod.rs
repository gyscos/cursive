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
}

type StyledHandle = OwningHandle<StringRef, Vec<Span<'static>>>;

/// A String that parses a markup language.
pub struct StyledString<M> {
    content: StyledHandle,
    _phantom: PhantomData<M>,
}

impl<M> StyledString<M>
where
    M: Markup,
{
    fn make_handle<S: Into<String>>(
        content: S
    ) -> Result<StyledHandle, M::Error> {
        let content = content.into();
        OwningHandle::try_new(StringRef::new(content), |input| {
            M::parse(unsafe { &*input })
        })
    }

    /// Creates a new styled string, parsing the given content.
    pub fn new<S>(content: S) -> Result<Self, M::Error>
    where
        S: Into<String>,
    {
        let content = Self::make_handle(content)?;
        Ok(StyledString {
            content,
            _phantom: PhantomData,
        })
    }

    /// Sets the content of this string.
    ///
    /// The content will be parsed; if an error is found,
    /// it will be returned here (and the content will be unchanged).
    pub fn set_content<S>(&mut self, content: S) -> Result<(), M::Error>
    where
        S: Into<String>,
    {
        self.content = Self::make_handle(content)?;

        Ok(())
    }

    /// Gives access to the parsed styled spans.
    pub fn spans<'a>(&'a self) -> &'a [Span<'a>] {
        &self.content
    }
}
