/// Extension trait for the `Cursive` root to simplify initialization.
///
/// It brings backend-specific methods to initialize a `Cursive` root.
///
/// # Examples
///
/// ```rust,no_run
/// use cursive::{Cursive, CursiveExt};
///
/// // Use `Cursive::default()` to pick one of the enabled backends,
/// // depending on cargo features.
/// let mut siv = Cursive::default();
///
/// // Or explicitly use a specific backend
/// #[cfg(feature = "ncurses-backend")]
/// let mut siv = Cursive::ncurses();
/// #[cfg(feature = "panncurses-backend")]
/// let mut siv = Cursive::pancurses();
/// #[cfg(feature = "termion-backend")]
/// let mut siv = Cursive::termion();
/// #[cfg(feature = "crossterm-backend")]
/// let mut siv = Cursive::crossterm();
/// #[cfg(feature = "blt-backend")]
/// let mut siv = Cursive::blt();
/// ```
pub trait CursiveExt {
    /// Type of the returned cursive root.
    type Cursive;

    /// Tries to use one of the enabled backends.
    ///
    /// Will fallback to the dummy backend if no other backend feature is enabled.
    ///
    /// # Panics
    ///
    /// If the backend initialization fails.
    fn default() -> Self::Cursive;

    /// Creates a new Cursive root using a ncurses backend.
    #[cfg(feature = "ncurses-backend")]
    fn ncurses() -> std::io::Result<Self::Cursive>;

    /// Creates a new Cursive root using a pancurses backend.
    #[cfg(feature = "pancurses-backend")]
    fn pancurses() -> std::io::Result<Self::Cursive>;

    /// Creates a new Cursive root using a termion backend.
    #[cfg(feature = "termion-backend")]
    fn termion() -> std::io::Result<Self::Cursive>;

    /// Creates a new Cursive root using a crossterm backend.
    #[cfg(feature = "crossterm-backend")]
    fn crossterm() -> Result<Self::Cursive, crossterm::ErrorKind>;

    /// Creates a new Cursive root using a bear-lib-terminal backend.
    #[cfg(feature = "blt-backend")]
    fn blt() -> Self::Cursive;
}

impl CursiveExt for cursive_core::Cursive {
    type Cursive = Self;

    fn default() -> Self::Cursive {
        cfg_if::cfg_if! {
            if #[cfg(feature = "blt-backend")] {
                Self::blt()
            } else if #[cfg(feature = "termion-backend")] {
                Self::termion().unwrap()
            } else if #[cfg(feature = "crossterm-backend")] {
                Self::crossterm().unwrap()
            } else if #[cfg(feature = "pancurses-backend")] {
                Self::pancurses().unwrap()
            } else if #[cfg(feature = "ncurses-backend")] {
                Self::ncurses().unwrap()
            } else {
                log::warn!("No built-it backend, falling back to Cursive::dummy().");
                Self::dummy()
            }
        }
    }

    #[cfg(feature = "ncurses-backend")]
    fn ncurses() -> std::io::Result<Self> {
        Self::try_new(crate::backends::curses::n::Backend::init)
    }

    #[cfg(feature = "pancurses-backend")]
    fn pancurses() -> std::io::Result<Self> {
        Self::try_new(crate::backends::curses::pan::Backend::init)
    }

    #[cfg(feature = "termion-backend")]
    fn termion() -> std::io::Result<Self> {
        Self::try_new(crate::backends::termion::Backend::init)
    }

    #[cfg(feature = "crossterm-backend")]
    fn crossterm() -> Result<Self, crossterm::ErrorKind> {
        Self::try_new(crate::backends::crossterm::Backend::init)
    }

    #[cfg(feature = "blt-backend")]
    fn blt() -> Self {
        Self::new(crate::backends::blt::Backend::init)
    }
}
