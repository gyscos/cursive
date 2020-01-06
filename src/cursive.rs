use std::any::Any;
use std::num::NonZeroU32;
#[cfg(feature = "toml")]
use std::path::Path;
use std::time::Duration;

use crossbeam_channel::{self, Receiver, Sender};

use crate::backend;
use crate::direction;
use crate::event::{Callback, Event, EventResult};
use crate::printer::Printer;
use crate::theme;
use crate::vec::Vec2;
use crate::view::{self, Finder, IntoBoxedView, Position, View};
use crate::views::{self, LayerPosition};

static DEBUG_VIEW_ID: &str = "_cursive_debug_view";

// How long we wait between two empty input polls
const INPUT_POLL_DELAY_MS: u64 = 30;

// Use AHash instead of the slower SipHash
type HashMap<K, V> = std::collections::HashMap<K, V, ahash::ABuildHasher>;

/// Central part of the cursive library.
///
/// It initializes ncurses on creation and cleans up on drop.
/// To use it, you should populate it with views, layouts, and callbacks,
/// then start the event loop with `run()`.
///
/// It uses a list of screen, with one screen active at a time.
pub struct Cursive {
    theme: theme::Theme,
    screens: Vec<views::StackView>,
    global_callbacks: HashMap<Event, Vec<Callback>>,
    menubar: views::Menubar,

    // Last layer sizes of the stack view.
    // If it changed, clear the screen.
    last_sizes: Vec<Vec2>,

    active_screen: ScreenId,

    running: bool,

    backend: Box<dyn backend::Backend>,

    cb_source: Receiver<Box<dyn FnOnce(&mut Cursive) + Send>>,
    cb_sink: Sender<Box<dyn FnOnce(&mut Cursive) + Send>>,

    // User-provided data.
    user_data: Box<dyn Any>,

    fps: Option<NonZeroU32>,
    boring_frame_count: u32,
}

/// Identifies a screen in the cursive root.
pub type ScreenId = usize;

/// Convenient alias to the result of `Cursive::cb_sink`.
///
/// # Notes
///
/// Callbacks need to be `Send`, which can be limiting in some cases.
///
/// In some case [`send_wrapper`] may help you work around that.
///
/// [`send_wrapper`]: https://crates.io/crates/send_wrapper
pub type CbSink = Sender<Box<dyn FnOnce(&mut Cursive) + Send>>;

cfg_if::cfg_if! {
    if #[cfg(feature = "blt-backend")] {
        impl Default for Cursive {
            fn default() -> Self {
                Self::blt()
            }
        }
    } else if #[cfg(feature = "termion-backend")] {
        impl Default for Cursive {
            fn default() -> Self {
                Self::termion().unwrap()
            }
        }
    } else if #[cfg(feature = "crossterm-backend")] {
        impl Default for Cursive {
            fn default() -> Self {
                Self::crossterm().unwrap()
            }
       }
    } else if #[cfg(feature = "pancurses-backend")] {
        impl Default for Cursive {
            fn default() -> Self {
                Self::pancurses().unwrap()
            }
        }
    } else if #[cfg(feature = "ncurses-backend")] {
        impl Default for Cursive {
            fn default() -> Self {
                Self::ncurses().unwrap()
            }
        }
    } else {
        impl Default for Cursive {
            fn default() -> Self {
                log::warn!("No built-it backend, falling back to Cursive::dummy().");
                Self::dummy()
            }
        }
    }
}

impl Cursive {
    /// Shortcut for `Cursive::try_new` with non-failible init function.
    ///
    /// You probably don't want to use this function directly. Instead,
    /// `Cursive::default()` or `Cursive::ncurses()` may be what you're
    /// looking for.
    pub fn new<F>(backend_init: F) -> Self
    where
        F: FnOnce() -> Box<dyn backend::Backend>,
    {
        Self::try_new::<_, ()>(|| Ok(backend_init())).unwrap()
    }

    /// Creates a new Cursive root, and initialize the back-end.
    ///
    /// * If you just want a cursive instance, use `Cursive::default()`.
    /// * If you want a specific backend, then:
    ///   * `Cursive::ncurses()` if the `ncurses-backend` feature is enabled (it is by default).
    ///   * `Cursive::pancurses()` if the `pancurses-backend` feature is enabled.
    ///   * `Cursive::termion()` if the `termion-backend` feature is enabled.
    ///   * `Cursive::crossterm()` if the `crossterm-backend` feature is enabled.
    ///   * `Cursive::blt()` if the `blt-backend` feature is enabled.
    ///   * `Cursive::dummy()` for a dummy backend, mostly useful for tests.
    /// * If you want to use a third-party backend, then `Cursive::new` is indeed the way to go:
    ///   * `Cursive::new(bring::your::own::Backend::new)`
    ///
    /// Examples:
    ///
    /// ```rust,no_run
    /// # use cursive::{Cursive, backend};
    /// let siv = Cursive::new(backend::dummy::Backend::init); // equivalent to Cursive::dummy()
    /// ```
    pub fn try_new<F, E>(backend_init: F) -> Result<Self, E>
    where
        F: FnOnce() -> Result<Box<dyn backend::Backend>, E>,
    {
        let theme = theme::load_default();

        let (cb_sink, cb_source) = crossbeam_channel::unbounded();

        backend_init().map(|backend| Cursive {
            theme,
            screens: vec![views::StackView::new()],
            last_sizes: Vec::new(),
            global_callbacks: HashMap::default(),
            menubar: views::Menubar::new(),
            active_screen: 0,
            running: true,
            cb_source,
            cb_sink,
            backend,
            fps: None,
            boring_frame_count: 0,
            user_data: Box::new(()),
        })
    }

    /// Creates a new Cursive root using a ncurses backend.
    #[cfg(feature = "ncurses-backend")]
    pub fn ncurses() -> std::io::Result<Self> {
        Self::try_new(backend::curses::n::Backend::init)
    }

    /// Creates a new Cursive root using a pancurses backend.
    #[cfg(feature = "pancurses-backend")]
    pub fn pancurses() -> std::io::Result<Self> {
        Self::try_new(backend::curses::pan::Backend::init)
    }

    /// Creates a new Cursive root using a termion backend.
    #[cfg(feature = "termion-backend")]
    pub fn termion() -> std::io::Result<Self> {
        Self::try_new(backend::termion::Backend::init)
    }

    /// Creates a new Cursive root using a crossterm backend.
    #[cfg(feature = "crossterm-backend")]
    pub fn crossterm() -> Result<Self, crossterm::ErrorKind> {
        Self::try_new(backend::crossterm::Backend::init)
    }

    /// Creates a new Cursive root using a bear-lib-terminal backend.
    #[cfg(feature = "blt-backend")]
    pub fn blt() -> Self {
        Self::new(backend::blt::Backend::init)
    }

    /// Creates a new Cursive root using a dummy backend.
    ///
    /// Nothing will be output. This is mostly here for tests.
    pub fn dummy() -> Self {
        Self::new(backend::dummy::Backend::init)
    }

    /// Sets some data to be stored in Cursive.
    ///
    /// It can later on be accessed with `Cursive::user_data()`
    pub fn set_user_data<T: Any>(&mut self, user_data: T) {
        self.user_data = Box::new(user_data);
    }

    /// Attempts to access the user-provided data.
    ///
    /// If some data was set previously with the same type, returns a
    /// reference to it.
    ///
    /// If nothing was set or if the type is different, returns `None`.
    pub fn user_data<T: Any>(&mut self) -> Option<&mut T> {
        self.user_data.downcast_mut()
    }

    /// Attemps to take by value the current user-data.
    ///
    /// If successful, this will replace the current user-data with the unit
    /// type `()`.
    ///
    /// If the current user data is not of the requested type, `None` will be
    /// returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut siv = cursive::Cursive::dummy();
    ///
    /// // Start with a simple `Vec<i32>` as user data.
    /// siv.set_user_data(vec![1i32, 2, 3]);
    /// assert_eq!(siv.user_data::<Vec<i32>>(), Some(&mut vec![1i32, 2, 3]));
    ///
    /// // Let's mutate the data a bit.
    /// siv.with_user_data(|numbers: &mut Vec<i32>| numbers.push(4));
    ///
    /// // If mutable reference is not enough, we can take the data by value.
    /// let data: Vec<i32> = siv.take_user_data().unwrap();
    /// assert_eq!(data, vec![1i32, 2, 3, 4]);
    ///
    /// // At this point the user data was removed and is no longer available.
    /// assert_eq!(siv.user_data::<Vec<i32>>(), None);
    /// ```
    pub fn take_user_data<T: Any>(&mut self) -> Option<T> {
        // Start by taking the user data and replacing it with a dummy.
        let user_data = std::mem::replace(&mut self.user_data, Box::new(()));

        // Downcast the data to the requested type.
        // If it works, unbox it.
        // It if doesn't, take it back.
        user_data
            .downcast()
            .map_err(|user_data| {
                // If we asked for the wrong type, put it back.
                self.user_data = user_data;
            })
            .map(|boxed| *boxed)
            .ok()
    }

    /// Runs the given closure on the stored user data, if any.
    ///
    /// If no user data was supplied, or if the type is different, nothing
    /// will be run.
    ///
    /// Otherwise, the result will be returned.
    pub fn with_user_data<F, T, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
        T: Any,
    {
        self.user_data().map(f)
    }

    /// Show the debug console.
    ///
    /// Currently, this will show logs if [`logger::init()`](crate::logger::init()) was called.
    pub fn show_debug_console(&mut self) {
        self.add_layer(
            views::Dialog::around(
                views::ScrollView::new(views::NamedView::new(
                    DEBUG_VIEW_ID,
                    views::DebugView::new(),
                ))
                .scroll_x(true),
            )
            .title("Debug console"),
        );
    }

    /// Show the debug console, or hide it if it's already visible.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::Cursive;
    /// # let mut siv = Cursive::dummy();
    /// siv.add_global_callback('~', Cursive::toggle_debug_console);
    /// ```
    pub fn toggle_debug_console(&mut self) {
        if let Some(pos) = self.screen_mut().find_layer_from_id(DEBUG_VIEW_ID)
        {
            self.screen_mut().remove_layer(pos);
        } else {
            self.show_debug_console();
        }
    }

    /// Returns a sink for asynchronous callbacks.
    ///
    /// Returns the sender part of a channel, that allows to send
    /// callbacks to `self` from other threads.
    ///
    /// Callbacks will be executed in the order
    /// of arrival on the next event cycle.
    ///
    /// # Notes
    ///
    /// Callbacks need to be `Send`, which can be limiting in some cases.
    ///
    /// In some case [`send_wrapper`] may help you work around that.
    ///
    /// [`send_wrapper`]: https://crates.io/crates/send_wrapper
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::*;
    /// let mut siv = Cursive::dummy();
    ///
    /// // quit() will be called during the next event cycle
    /// siv.cb_sink().send(Box::new(|s| s.quit())).unwrap();
    /// ```
    pub fn cb_sink(&self) -> &CbSink {
        &self.cb_sink
    }

    /// Selects the menubar.
    pub fn select_menubar(&mut self) {
        self.menubar.take_focus(direction::Direction::none());
    }

    /// Sets the menubar autohide feature.
    ///
    /// * When enabled (default), the menu is only visible when selected.
    /// * When disabled, the menu is always visible and reserves the top row.
    pub fn set_autohide_menu(&mut self, autohide: bool) {
        self.menubar.autohide = autohide;
    }

    /// Access the menu tree used by the menubar.
    ///
    /// This allows to add menu items to the menubar.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::{Cursive, event};
    /// # use cursive::views::{Dialog};
    /// # use cursive::traits::*;
    /// # use cursive::menu::*;
    /// #
    /// let mut siv = Cursive::dummy();
    ///
    /// siv.menubar()
    ///    .add_subtree("File",
    ///         MenuTree::new()
    ///             .leaf("New", |s| s.add_layer(Dialog::info("New file!")))
    ///             .subtree("Recent", MenuTree::new().with(|tree| {
    ///                 for i in 1..100 {
    ///                     tree.add_leaf(format!("Item {}", i), |_| ())
    ///                 }
    ///             }))
    ///             .delimiter()
    ///             .with(|tree| {
    ///                 for i in 1..10 {
    ///                     tree.add_leaf(format!("Option {}", i), |_| ());
    ///                 }
    ///             })
    ///             .delimiter()
    ///             .leaf("Quit", |s| s.quit()))
    ///    .add_subtree("Help",
    ///         MenuTree::new()
    ///             .subtree("Help",
    ///                      MenuTree::new()
    ///                          .leaf("General", |s| {
    ///                              s.add_layer(Dialog::info("Help message!"))
    ///                          })
    ///                          .leaf("Online", |s| {
    ///                              s.add_layer(Dialog::info("Online help?"))
    ///                          }))
    ///             .leaf("About",
    ///                   |s| s.add_layer(Dialog::info("Cursive v0.0.0"))));
    ///
    /// siv.add_global_callback(event::Key::Esc, |s| s.select_menubar());
    /// ```
    pub fn menubar(&mut self) -> &mut views::Menubar {
        &mut self.menubar
    }

    /// Returns the currently used theme.
    pub fn current_theme(&self) -> &theme::Theme {
        &self.theme
    }

    /// Sets the current theme.
    pub fn set_theme(&mut self, theme: theme::Theme) {
        self.theme = theme;
        self.clear();
    }

    /// Clears the screen.
    ///
    /// Users rarely have to call this directly.
    pub fn clear(&mut self) {
        self.backend
            .clear(self.theme.palette[theme::PaletteColor::Background]);
    }

    #[cfg(feature = "toml")]
    /// Loads a theme from the given file.
    ///
    /// `filename` must point to a valid toml file.
    ///
    /// Must have the `toml` feature enabled.
    pub fn load_theme_file<P: AsRef<Path>>(
        &mut self,
        filename: P,
    ) -> Result<(), theme::Error> {
        theme::load_theme_file(filename).map(|theme| self.set_theme(theme))
    }

    #[cfg(feature = "toml")]
    /// Loads a theme from the given string content.
    ///
    /// Content must be valid toml.
    ///
    /// Must have the `toml` feature enabled.
    pub fn load_toml(&mut self, content: &str) -> Result<(), theme::Error> {
        theme::load_toml(content).map(|theme| self.set_theme(theme))
    }

    /// Sets the refresh rate, in frames per second.
    ///
    /// Note that the actual frequency is not guaranteed.
    ///
    /// Between 0 and 30. Call with `fps = 0` to disable (default value).
    pub fn set_fps(&mut self, fps: u32) {
        self.fps = NonZeroU32::new(fps);
    }

    /// Enables or disables automatic refresh of the screen.
    ///
    /// This is a shortcut to call `set_fps` with `30` or `0` depending on
    /// `autorefresh`.
    pub fn set_autorefresh(&mut self, autorefresh: bool) {
        self.set_fps(if autorefresh { 30 } else { 0 });
    }

    /// Returns a reference to the currently active screen.
    pub fn screen(&self) -> &views::StackView {
        let id = self.active_screen;
        &self.screens[id]
    }

    /// Returns a mutable reference to the currently active screen.
    pub fn screen_mut(&mut self) -> &mut views::StackView {
        let id = self.active_screen;
        &mut self.screens[id]
    }

    /// Returns the id of the currently active screen.
    pub fn active_screen(&self) -> ScreenId {
        self.active_screen
    }

    /// Adds a new screen, and returns its ID.
    pub fn add_screen(&mut self) -> ScreenId {
        let res = self.screens.len();
        self.screens.push(views::StackView::new());
        res
    }

    /// Convenient method to create a new screen, and set it as active.
    pub fn add_active_screen(&mut self) -> ScreenId {
        let res = self.add_screen();
        self.set_screen(res);
        res
    }

    /// Sets the active screen. Panics if no such screen exist.
    pub fn set_screen(&mut self, screen_id: ScreenId) {
        if screen_id >= self.screens.len() {
            panic!(
                "Tried to set an invalid screen ID: {}, but only {} \
                 screens present.",
                screen_id,
                self.screens.len()
            );
        }
        self.active_screen = screen_id;
    }

    /// Tries to find the view pointed to by the given selector.
    ///
    /// Runs a closure on the view once it's found, and return the
    /// result.
    ///
    /// If the view is not found, or if it is not of the asked type,
    /// returns None.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::{Cursive, views, view};
    /// # use cursive::traits::*;
    /// let mut siv = Cursive::dummy();
    ///
    /// siv.add_layer(views::TextView::new("Text #1").with_id("text"));
    ///
    /// siv.add_global_callback('p', |s| {
    ///     s.call_on(
    ///         &view::Selector::Id("text"),
    ///         |view: &mut views::TextView| {
    ///             view.set_content("Text #2");
    ///         },
    ///     );
    /// });
    /// ```
    pub fn call_on<V, F, R>(
        &mut self,
        sel: &view::Selector<'_>,
        callback: F,
    ) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        self.screen_mut().call_on(sel, callback)
    }

    /// Tries to find the view identified by the given id.
    ///
    /// Convenient method to use `call_on` with a `view::Selector::Id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::{Cursive, views};
    /// # use cursive::traits::*;
    /// let mut siv = Cursive::dummy();
    ///
    /// siv.add_layer(views::TextView::new("Text #1")
    ///                               .with_id("text"));
    ///
    /// siv.add_global_callback('p', |s| {
    ///     s.call_on_id("text", |view: &mut views::TextView| {
    ///         view.set_content("Text #2");
    ///     });
    /// });
    /// ```
    pub fn call_on_id<V, F, R>(&mut self, id: &str, callback: F) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        self.call_on(&view::Selector::Name(id), callback)
    }

    /// Convenient method to find a view wrapped in [`NamedView`].
    ///
    /// This looks for a `NamedView<V>` with the given ID, and return
    /// a [`ViewRef`] to the wrapped view. The `ViewRef` implements
    /// `DerefMut<Target=T>`, so you can treat it just like a `&mut T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::Cursive;
    /// # use cursive::views::{TextView, ViewRef};
    /// # let mut siv = Cursive::dummy();
    /// use cursive::traits::Identifiable;
    ///
    /// siv.add_layer(TextView::new("foo").with_id("id"));
    ///
    /// // Could be called in a callback
    /// let mut view: ViewRef<TextView> = siv.find_id("id").unwrap();
    /// view.set_content("bar");
    /// ```
    ///
    /// Note that you must specify the exact type for the view you're after; for example, using the
    /// wrong item type in a `SelectView` will not find anything:
    ///
    /// ```rust
    /// # use cursive::Cursive;
    /// # use cursive::views::{SelectView};
    /// # let mut siv = Cursive::dummy();
    /// use cursive::traits::Identifiable;
    ///
    /// let select = SelectView::new().item("zero", 0u32).item("one", 1u32);
    /// siv.add_layer(select.with_id("select"));
    ///
    /// // Specifying a wrong type will not return anything.
    /// assert!(siv.find_id::<SelectView<String>>("select").is_none());
    ///
    /// // Omitting the type will use the default type, in this case `String`.
    /// assert!(siv.find_id::<SelectView>("select").is_none());
    ///
    /// // But with the correct type, it works fine.
    /// assert!(siv.find_id::<SelectView<u32>>("select").is_some());
    /// ```
    ///
    /// [`NamedView`]: views/struct.NamedView.html
    /// [`ViewRef`]: views/type.ViewRef.html
    pub fn find_id<V>(&mut self, id: &str) -> Option<views::ViewRef<V>>
    where
        V: View + Any,
    {
        self.call_on_id(id, views::NamedView::<V>::get_mut)
    }

    /// Moves the focus to the view identified by `id`.
    ///
    /// Convenient method to call `focus` with a `view::Selector::Id`.
    pub fn focus_id(&mut self, id: &str) -> Result<(), ()> {
        self.focus(&view::Selector::Name(id))
    }

    /// Moves the focus to the view identified by `sel`.
    pub fn focus(&mut self, sel: &view::Selector<'_>) -> Result<(), ()> {
        self.screen_mut().focus_view(sel)
    }

    /// Adds a global callback.
    ///
    /// Will be triggered on the given key press when no view catches it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::*;
    /// let mut siv = Cursive::dummy();
    ///
    /// siv.add_global_callback('q', |s| s.quit());
    /// ```
    pub fn add_global_callback<F, E: Into<Event>>(&mut self, event: E, cb: F)
    where
        F: FnMut(&mut Cursive) + 'static,
    {
        self.global_callbacks
            .entry(event.into())
            .or_insert_with(Vec::new)
            .push(Callback::from_fn_mut(cb));
    }

    /// Removes any callback tied to the given event.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive::Cursive;
    /// let mut siv = Cursive::dummy();
    ///
    /// siv.add_global_callback('q', |s| s.quit());
    /// siv.clear_global_callbacks('q');
    /// ```
    pub fn clear_global_callbacks<E>(&mut self, event: E)
    where
        E: Into<Event>,
    {
        let event = event.into();
        self.global_callbacks.remove(&event);
    }

    /// Add a layer to the current screen.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive::{Cursive, views};
    /// let mut siv = Cursive::dummy();
    ///
    /// siv.add_layer(views::TextView::new("Hello world!"));
    /// ```
    pub fn add_layer<T>(&mut self, view: T)
    where
        T: IntoBoxedView,
    {
        self.screen_mut().add_layer(view);
    }

    /// Adds a new full-screen layer to the current screen.
    ///
    /// Fullscreen layers have no shadow.
    pub fn add_fullscreen_layer<T>(&mut self, view: T)
    where
        T: IntoBoxedView,
    {
        self.screen_mut().add_fullscreen_layer(view);
    }

    /// Convenient method to remove a layer from the current screen.
    pub fn pop_layer(&mut self) -> Option<Box<dyn View>> {
        self.screen_mut().pop_layer()
    }

    /// Convenient stub forwarding layer repositioning.
    pub fn reposition_layer(
        &mut self,
        layer: LayerPosition,
        position: Position,
    ) {
        self.screen_mut().reposition_layer(layer, position);
    }

    // Handles a key event when it was ignored by the current view
    fn on_ignored_event(&mut self, event: Event) {
        let cb_list = match self.global_callbacks.get(&event) {
            None => return,
            Some(cb_list) => cb_list.clone(),
        };
        // Not from a view, so no viewpath here
        for cb in cb_list {
            cb(self);
        }
    }

    /// Processes an event.
    ///
    /// * If the menubar is active, it will be handled the event.
    /// * The view tree will be handled the event.
    /// * If ignored, global_callbacks will be checked for this event.
    pub fn on_event(&mut self, event: Event) {
        if event == Event::Exit {
            self.quit();
        }

        if event == Event::WindowResize {
            self.clear();
        }

        if let Event::Mouse {
            event, position, ..
        } = event
        {
            if event.grabs_focus()
                && !self.menubar.autohide
                && !self.menubar.has_submenu()
                && position.y == 0
            {
                self.select_menubar();
            }
        }

        // Event dispatch order:
        // * Focused element:
        //     * Menubar (if active)
        //     * Current screen (top layer)
        // * Global callbacks
        if self.menubar.receive_events() {
            self.menubar.on_event(event).process(self);
        } else {
            let offset = if self.menubar.autohide { 0 } else { 1 };
            match self.screen_mut().on_event(event.relativized((0, offset))) {
                // If the event was ignored,
                // it is our turn to play with it.
                EventResult::Ignored => self.on_ignored_event(event),
                EventResult::Consumed(None) => (),
                EventResult::Consumed(Some(cb)) => cb(self),
            }
        }
    }

    /// Returns the size of the screen, in characters.
    pub fn screen_size(&self) -> Vec2 {
        self.backend.screen_size()
    }

    fn layout(&mut self) {
        let size = self.screen_size();
        let offset = if self.menubar.autohide { 0 } else { 1 };
        let size = size.saturating_sub((0, offset));
        self.screen_mut().layout(size);
    }

    fn draw(&mut self) {
        let sizes = self.screen().layer_sizes();
        if self.last_sizes != sizes {
            self.clear();
            self.last_sizes = sizes;
        }

        let printer =
            Printer::new(self.screen_size(), &self.theme, &*self.backend);

        let selected = self.menubar.receive_events();

        // Print the stackview background before the menubar
        let offset = if self.menubar.autohide { 0 } else { 1 };
        let id = self.active_screen;
        let sv_printer = printer.offset((0, offset)).focused(!selected);

        self.screens[id].draw_bg(&sv_printer);

        // Draw the currently active screen
        // If the menubar is active, nothing else can be.
        // Draw the menubar?
        if self.menubar.visible() {
            let printer = printer.focused(self.menubar.receive_events());
            self.menubar.draw(&printer);
        }

        // finally draw stackview layers
        // using variables from above
        self.screens[id].draw_fg(&sv_printer);
    }

    /// Returns `true` until [`quit(&mut self)`] is called.
    ///
    /// [`quit(&mut self)`]: #method.quit
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Runs the event loop.
    ///
    /// It will wait for user input (key presses)
    /// and trigger callbacks accordingly.
    ///
    /// Internally, it calls [`step(&mut self)`] until [`quit(&mut self)`] is
    /// called.
    ///
    /// After this function returns, you can call it again and it will start a
    /// new loop.
    ///
    /// [`step(&mut self)`]: #method.step
    /// [`quit(&mut self)`]: #method.quit
    pub fn run(&mut self) {
        self.running = true;

        self.refresh();

        // And the big event loop begins!
        while self.running {
            self.step();
        }
    }

    /// Performs a single step from the event loop.
    ///
    /// Useful if you need tighter control on the event loop.
    /// Otherwise, [`run(&mut self)`] might be more convenient.
    ///
    /// Returns `true` if an input event or callback was received
    /// during this step, and `false` otherwise.
    ///
    /// [`run(&mut self)`]: #method.run
    pub fn step(&mut self) -> bool {
        let received_something = self.process_events();
        self.post_events(received_something);
        received_something
    }

    /// Performs the first half of `Self::step()`.
    ///
    /// This is an advanced method for fine-tuned manual stepping;
    /// you probably want [`run`][1] or [`step`][2].
    ///
    /// This processes any pending event or callback. After calling this,
    /// you will want to call [`post_events`][3] with the result from this
    /// function.
    ///
    /// Returns `true` if an event or callback was received,
    /// and `false` otherwise.
    ///
    /// [1]: Cursive::run()
    /// [2]: Cursive::step()
    /// [3]: Cursive::post_events()
    pub fn process_events(&mut self) -> bool {
        // Things are boring if nothing significant happened.
        let mut boring = true;

        // First, handle all available input
        while let Some(event) = self.backend.poll_event() {
            boring = false;
            self.on_event(event);

            if !self.running {
                return true;
            }
        }

        // Then, handle any available callback
        while let Ok(cb) = self.cb_source.try_recv() {
            boring = false;
            cb(self);

            if !self.running {
                return true;
            }
        }

        !boring
    }

    /// Performs the second half of `Self::step()`.
    ///
    /// This is an advanced method for fine-tuned manual stepping;
    /// you probably want [`run`][1] or [`step`][2].
    ///
    /// You should call this after [`process_events`][3].
    ///
    /// [1]: Cursive::run()
    /// [2]: Cursive::step()
    /// [3]: Cursive::process_events()
    pub fn post_events(&mut self, received_something: bool) {
        let boring = !received_something;
        // How many times should we try if it's still boring?
        // Total duration will be INPUT_POLL_DELAY_MS * repeats
        // So effectively fps = 1000 / INPUT_POLL_DELAY_MS / repeats
        if !boring
            || self
                .fps
                .map(|fps| 1000 / INPUT_POLL_DELAY_MS as u32 / fps.get())
                .map(|repeats| self.boring_frame_count >= repeats)
                .unwrap_or(false)
        {
            // We deserve to draw something!

            if boring {
                // We're only here because of a timeout.
                self.on_event(Event::Refresh);
            }

            self.refresh();
        }

        if boring {
            std::thread::sleep(Duration::from_millis(INPUT_POLL_DELAY_MS));
            self.boring_frame_count += 1;
        }
    }

    /// Refresh the screen with the current view tree state.
    pub fn refresh(&mut self) {
        self.boring_frame_count = 0;

        // Do we need to redraw everytime?
        // Probably, actually.
        // TODO: Do we need to re-layout everytime?
        self.layout();

        // TODO: Do we need to redraw every view every time?
        // (Is this getting repetitive? :p)
        self.draw();
        self.backend.refresh();
    }

    /// Stops the event loop.
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Does not do anything.
    pub fn noop(&mut self) {
        // foo
    }

    /// Return the name of the backend used.
    ///
    /// Mostly used for debugging.
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }
}

impl Drop for Cursive {
    fn drop(&mut self) {
        self.backend.finish();
    }
}
