/// Generic trait to enable chainable API
pub trait With: Sized {
    /// Calls the given closure and return the result.
    ///
    /// Used to chainify wrapper constructors.
    fn wrap_with<U, F: FnOnce(Self) -> U>(self, f: F) -> U {
        f(self)
    }

    /// Calls the given closure on `self`.
    #[must_use]
    fn with<F: FnOnce(&mut Self)>(mut self, f: F) -> Self {
        f(&mut self);
        self
    }

    /// Calls the given closure on `self`.
    fn try_with<E, F>(mut self, f: F) -> Result<Self, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        f(&mut self)?;
        Ok(self)
    }

    /// Calls the given closure if `condition == true`.
    #[must_use]
    fn with_if<F>(mut self, condition: bool, f: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        if condition {
            f(&mut self);
        }
        self
    }
}

impl<T: Sized> With for T {}
