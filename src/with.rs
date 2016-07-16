
/// Generic trait to enable chainable API
pub trait With: Sized {
    /// Calls the given closure on `self`.
    fn with<F: FnOnce(&mut Self)>(mut self, f: F) -> Self {
        f(&mut self);
        self
    }
}

impl<T: Sized> With for T {}
