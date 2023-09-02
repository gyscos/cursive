
use async_trait::async_trait;

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
/// #[cfg(feature = "wasm-backend")]
/// siv.run_wasm();
/// ```
#[async_trait(?Send)]
pub trait CursiveExt {
    /// Tries to use one of the enabled backends.
    ///
    /// Will fallback to the dummy backend if no other backend feature is enabled.
    ///
    /// # Panics
    ///
    /// If the backend initialization fails.
    async fn run(&mut self);

    /// Creates a new Cursive root using a ncurses backend.
    #[cfg(feature = "ncurses-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "ncurses-backend")))]
    async fn run_ncurses(&mut self) -> std::io::Result<()>;

    /// Creates a new Cursive root using a pancurses backend.
    #[cfg(feature = "pancurses-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "pancurses-backend")))]
    async fn run_pancurses(&mut self) -> std::io::Result<()>;

    /// Creates a new Cursive root using a termion backend.
    #[cfg(feature = "termion-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "termion-backend")))]
    async fn run_termion(&mut self) -> std::io::Result<()>;

    /// Creates a new Cursive root using a crossterm backend.
    #[cfg(feature = "crossterm-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "crossterm-backend")))]
    async fn run_crossterm(&mut self) -> Result<(), crossterm::ErrorKind>;

    /// Creates a new Cursive root using a bear-lib-terminal backend.
    #[cfg(feature = "blt-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "blt-backend")))]
    async fn run_blt(&mut self);

    /// Creates a new Cursive root using a wasm backend.
    #[cfg(feature = "wasm-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "wasm-backend")))]
    async fn run_wasm(&mut self) -> Result<(), std::io::Error>;
}

#[async_trait(?Send)]
impl CursiveExt for cursive_core::Cursive {
    async fn run(&mut self) {
        cfg_if::cfg_if! {
            if #[cfg(feature = "blt-backend")] {
                self.run_blt().await
            } else if #[cfg(feature = "termion-backend")] {
                self.run_termion().await.unwrap()
            } else if #[cfg(feature = "crossterm-backend")] {
                self.run_crossterm().await.unwrap()
            } else if #[cfg(feature = "pancurses-backend")] {
                self.run_pancurses().await.unwrap()
            } else if #[cfg(feature = "ncurses-backend")] {
                self.run_ncurses().await.unwrap()
            } else if #[cfg(feature = "wasm-backend")] {
                self.run_wasm().await.unwrap()
            } else {
                log::warn!("No built-it backend, falling back to Cursive::dummy().");
                self.run_dummy().await
            }
        }
    }

    #[cfg(feature = "ncurses-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "curses-backend")))]
    async fn run_ncurses(&mut self) -> std::io::Result<()> {
        self.try_run_with(crate::backends::curses::n::Backend::init).await
    }

    #[cfg(feature = "pancurses-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "pancurses-backend")))]
    async fn run_pancurses(&mut self) -> std::io::Result<()> {
        self.try_run_with(crate::backends::curses::pan::Backend::init).await
    }

    #[cfg(feature = "termion-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "termion-backend")))]
    async fn run_termion(&mut self) -> std::io::Result<()> {
        self.try_run_with(crate::backends::termion::Backend::init).await
    }

    #[cfg(feature = "crossterm-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "crossterm-backend")))]
    async fn run_crossterm(&mut self) -> Result<(), crossterm::ErrorKind> {
        self.try_run_with(crate::backends::crossterm::Backend::init).await
    }

    #[cfg(feature = "blt-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "blt-backend")))]
    async fn run_blt(&mut self) {
        self.run_with(crate::backends::blt::Backend::init).await
    }

    #[cfg(feature = "wasm-backend")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "wasm-backend")))]
    async fn run_wasm(&mut self) -> Result<(), std::io::Error> {
        self.try_run_with(crate::backends::wasm::Backend::init).await
    }
}
