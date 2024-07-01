/// Extension trait for the `Cursive` root to simplify initialization.
///
/// It brings backend-specific methods to initialize a `Cursive` root.
///
/// # Examples
///
/// ```rust,no_run
/// use cursive::{Cursive, CursiveExt};
///
/// let mut siv = Cursive::new();
///
/// // Use `CursiveExt::run()` to pick one of the enabled backends,
/// // depending on cargo features.
/// siv.run();
///
/// // Or explicitly use a specific backend
/// #[cfg(feature = "ncurses-backend")]
/// siv.run_ncurses().unwrap();
/// #[cfg(feature = "panncurses-backend")]
/// siv.run_pancurses().unwrap();
/// #[cfg(feature = "termion-backend")]
/// siv.run_termion().unwrap();
/// #[cfg(feature = "crossterm-backend")]
/// siv.run_crossterm().unwrap();
/// #[cfg(feature = "blt-backend")]
/// siv.run_blt();
/// ```
pub trait CursiveExt {
    /// Tries to use one of the enabled backends.
    ///
    /// Will fallback to the dummy backend if no other backend feature is enabled.
    ///
    /// # Panics
    ///
    /// If the backend initialization fails.
    fn run(&mut self);

    /// Creates a new Cursive root using a ncurses backend.
    #[cfg(feature = "ncurses-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "ncurses-backend")))]
    fn run_ncurses(&mut self) -> std::io::Result<()>;

    /// Creates a new Cursive root using a pancurses backend.
    #[cfg(feature = "pancurses-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "pancurses-backend")))]
    fn run_pancurses(&mut self) -> std::io::Result<()>;

    /// Creates a new Cursive root using a termion backend.
    #[cfg(feature = "termion-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "termion-backend")))]
    fn run_termion(&mut self) -> std::io::Result<()>;

    /// Creates a new Cursive root using a crossterm backend.
    #[cfg(feature = "crossterm-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "crossterm-backend")))]
    fn run_crossterm(&mut self) -> Result<(), std::io::Error>;

    /// Creates a new Cursive root using a bear-lib-terminal backend.
    #[cfg(feature = "blt-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "blt-backend")))]
    fn run_blt(&mut self);
}

impl CursiveExt for cursive_core::Cursive {
    fn run(&mut self) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "blt-backend")] {
                self.run_blt()
            } else if #[cfg(feature = "termion-backend")] {
                self.run_termion().unwrap()
            } else if #[cfg(feature = "pancurses-backend")] {
                self.run_pancurses().unwrap()
            } else if #[cfg(feature = "ncurses-backend")] {
                self.run_ncurses().unwrap()
            } else if #[cfg(feature = "crossterm-backend")] {
                self.run_crossterm().unwrap()
            } else {
                log::warn!("No built-it backend, falling back to Cursive::dummy().");
                self.run_dummy()
            }
        }
    }

    #[cfg(feature = "ncurses-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "curses-backend")))]
    fn run_ncurses(&mut self) -> std::io::Result<()> {
        self.try_run_with(crate::backends::curses::n::Backend::init)
    }

    #[cfg(feature = "pancurses-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "pancurses-backend")))]
    fn run_pancurses(&mut self) -> std::io::Result<()> {
        self.try_run_with(crate::backends::curses::pan::Backend::init)
    }

    #[cfg(feature = "termion-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "termion-backend")))]
    fn run_termion(&mut self) -> std::io::Result<()> {
        self.try_run_with(crate::backends::termion::Backend::init)
    }

    #[cfg(feature = "crossterm-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "crossterm-backend")))]
    fn run_crossterm(&mut self) -> Result<(), std::io::Error> {
        self.try_run_with(crate::backends::crossterm::Backend::init)
    }

    #[cfg(feature = "blt-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "blt-backend")))]
    fn run_blt(&mut self) {
        self.run_with(crate::backends::blt::Backend::init)
    }
}
