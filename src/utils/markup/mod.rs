//! Parse various text markup formats.
//!
//! Each module is optional and relies on a feature.

#[cfg(feature = "markdown")]
pub mod markdown;

#[cfg(feature = "markdown")]
pub use self::markdown::MarkdownText;
use owning_ref::OwningHandle;
use owning_ref::StringRef;
use std::borrow::Cow;
use std::ops::Deref;
use theme::Style;
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

/// Thin wrapper around a string, with a markup format.
///
/// This only wraps the text and indicates how it should be parsed;
/// it does not parse the text itself.
pub trait MarkupText {
    /// Markup format to use to parse the string.
    type M: Markup;

    /// Access the inner string.
    fn to_string(self) -> String;
}

/// Unwrapped text gets the "Plain" markup for free.
impl<S: Into<String>> MarkupText for S {
    type M = Plain;

    fn to_string(self) -> String {
        self.into()
    }
}

/// Dummy `Markup` implementation that returns the text as-is.
pub struct Plain;

impl Markup for Plain {
    type Error = ();

    fn parse<'a>(input: &'a str) -> Result<Vec<Span<'a>>, Self::Error> {
        Ok(if input.is_empty() {
            Vec::new()
        } else {
            vec![
                Span {
                    text: Cow::Borrowed(input),
                    style: Style::none(),
                },
            ]
        })
    }
}

/// Holds both parsed spans, and the input string they borrow.
///
/// This is used to pass around a parsed string.
pub type StyledHandle = OwningHandle<StringRef, Vec<Span<'static>>>;

/// A String that parses a markup language.
pub struct StyledString {
    content: Option<StyledHandle>,
}

impl StyledString {
    /// Creates a new styled string, parsing the given content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::utils::markup::StyledString;
    /// let styled_string = StyledString::new("*plain* text");
    /// ```
    pub fn new<T>(content: T) -> Result<Self, <T::M as Markup>::Error>
    where
        T: MarkupText,
    {
        let content = content.to_string();

        let content = Some(T::M::make_handle(content)?);

        Ok(StyledString { content })
    }

    /// Returns a plain StyledString without any style.
    ///
    /// > You got no style, Dutch. You know that.
    pub fn plain<S>(content: S) -> Self
    where
        S: Into<String>,
    {
        Self::new(content).unwrap()
    }

    /// Sets the content of this string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::utils::markup::StyledString;
    /// # let mut styled_string = StyledString::new("").unwrap();
    /// styled_string.set_content("*plain* text").unwrap();
    /// ```
    pub fn set_content<T>(
        &mut self, content: T
    ) -> Result<(), <<T as MarkupText>::M as Markup>::Error>
    where
        T: MarkupText,
    {
        let content = content.to_string();

        self.content = Some(T::M::make_handle(content)?);

        Ok(())
    }

    /// Sets the content of this string to plain text.
    pub fn set_plain<S>(&mut self, content: S)
    where
        S: Into<String>,
    {
        self.set_content(content).unwrap();
    }

    /// Append `content` to the end.
    ///
    /// Re-parse everything after.
    pub fn append_content<T>(
        &mut self, content: T
    ) -> Result<(), <T::M as Markup>::Error>
    where
        T: MarkupText,
    {
        self.with_content::<T::M, _, _>(|c| c.push_str(&content.to_string()))
    }

    /// Run a closure on the text content.
    ///
    /// And re-parse everything after.
    pub fn with_content<M, F, O>(&mut self, f: F) -> Result<O, M::Error>
    where
        M: Markup,
        F: FnOnce(&mut String) -> O,
    {
        // Get hold of the StyledHandle
        let content = self.content.take().unwrap();
        // Get the inner String
        let mut content = content.into_inner().into_inner();
        // Do what we have to do
        let out = f(&mut content);
        // And re-parse everything
        self.content = Some(M::make_handle(content)?);

        Ok(out)
    }

    /// Gives access to the parsed styled spans.
    pub fn spans<'a>(&'a self) -> &'a [Span<'a>] {
        &self.content.as_ref().unwrap()
    }
}

impl Deref for StyledString {
    type Target = str;

    fn deref(&self) -> &str {
        &self.content.as_ref().unwrap().owner()
    }
}
