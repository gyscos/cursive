use backend;
use backend::Backend;
use direction;
use event::{Callback, Event, EventResult};
use printer::Printer;
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use theme;
use vec::Vec2;
use view::{self, Finder, View};
use views;

/// Identifies a screen in the cursive root.
pub type ScreenId = usize;

/// Central part of the cursive library.
///
/// It initializes ncurses on creation and cleans up on drop.
/// To use it, you should populate it with views, layouts and callbacks,
/// then start the event loop with run().
///
/// It uses a list of screen, with one screen active at a time.
pub struct Cursive {
    theme: theme::Theme,
    screens: Vec<views::StackView>,
    global_callbacks: HashMap<Event, Callback>,
    menubar: views::Menubar,

    // Last layer sizes of the stack view.
    // If it changed, clear the screen.
    last_sizes: Vec<Vec2>,

    active_screen: ScreenId,

    running: bool,

    backend: backend::Concrete,

    cb_source: mpsc::Receiver<Box<Fn(&mut Cursive) + Send>>,
    cb_sink: mpsc::Sender<Box<Fn(&mut Cursive) + Send>>,
}

new_default!(Cursive);

impl Cursive {
    /// Creates a new Cursive root, and initialize the back-end.
    pub fn new() -> Self {
        let backend = backend::Concrete::init();

        let theme = theme::load_default();
        // theme.activate(&mut backend);
        // let theme = theme::load_theme("assets/style.toml").unwrap();

        let (tx, rx) = mpsc::channel();

        let mut res = Cursive {
            theme: theme,
            screens: Vec::new(),
            last_sizes: Vec::new(),
            global_callbacks: HashMap::new(),
            menubar: views::Menubar::new(),
            active_screen: 0,
            running: true,
            cb_source: rx,
            cb_sink: tx,
            backend: backend,
        };

        res.screens.push(views::StackView::new());

        res
    }

    /// Returns a sink for asynchronous callbacks.
    ///
    /// Returns the sender part of a channel, that allows to send
    /// callbacks to `self` from other threads.
    ///
    /// Callbacks will be executed in the order
    /// of arrival on the next event cycle.
    ///
    /// Note that you currently need to call [`set_fps`] to force cursive to
    /// regularly check for messages.
    ///
    /// [`set_fps`]: #method.set_fps
    pub fn cb_sink(&self) -> &mpsc::Sender<Box<Fn(&mut Cursive) + Send>> {
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
    /// ```no_run
    /// # extern crate cursive;
    /// #
    /// # use cursive::{Cursive, event};
    /// # use cursive::views::{Dialog};
    /// # use cursive::traits::*;
    /// # use cursive::menu::*;
    /// #
    /// # fn main() {
    /// let mut siv = Cursive::new();
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
    /// # }
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
    pub fn clear(&self) {
        self.backend.clear(self.theme.colors.background);
    }

    /// Loads a theme from the given file.
    ///
    /// `filename` must point to a valid toml file.
    pub fn load_theme_file<P: AsRef<Path>>(
        &mut self, filename: P
    ) -> Result<(), theme::Error> {
        self.set_theme(try!(theme::load_theme_file(filename)));
        Ok(())
    }

    /// Loads a theme from the given string content.
    ///
    /// Content must be valid toml.
    pub fn load_theme(&mut self, content: &str) -> Result<(), theme::Error> {
        self.set_theme(try!(theme::load_theme(content)));
        Ok(())
    }

    /// Sets the refresh rate, in frames per second.
    ///
    /// Regularly redraws everything, even when no input is given.
    ///
    /// You currently need this to regularly check
    /// for events sent using [`cb_sink`].
    ///
    /// Between 0 and 1000. Call with `fps = 0` to disable (default value).
    ///
    /// [`cb_sink`]: #method.cb_sink
    pub fn set_fps(&mut self, fps: u32) {
        self.backend.set_refresh_rate(fps)
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
    /// ```no_run
    /// # extern crate cursive;
    /// # use cursive::{Cursive, views, view};
    /// # use cursive::traits::*;
    /// # fn main() {
    /// let mut siv = Cursive::new();
    ///
    /// siv.add_layer(views::TextView::new("Text #1")
    ///                               .with_id("text"));
    ///
    /// siv.add_global_callback('p', |s| {
    ///     s.call_on(&view::Selector::Id("text"), |view: &mut views::TextView| {
    ///         view.set_content("Text #2");
    ///     });
    /// });
    /// # }
    /// ```
    pub fn call_on<V, F, R>(
        &mut self, sel: &view::Selector, callback: F
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
    /// ```no_run
    /// # extern crate cursive;
    /// # use cursive::{Cursive, views};
    /// # use cursive::traits::*;
    /// # fn main() {
    /// let mut siv = Cursive::new();
    ///
    /// siv.add_layer(views::TextView::new("Text #1")
    ///                               .with_id("text"));
    ///
    /// siv.add_global_callback('p', |s| {
    ///     s.call_on_id("text", |view: &mut views::TextView| {
    ///         view.set_content("Text #2");
    ///     });
    /// });
    /// # }
    /// ```
    pub fn call_on_id<V, F, R>(&mut self, id: &str, callback: F) -> Option<R>
    where
        V: View + Any,
        F: FnOnce(&mut V) -> R,
    {
        self.call_on(&view::Selector::Id(id), callback)
    }

    /// Convenient method to find a view wrapped in [`IdView`].
    ///
    /// This looks for a `IdView<V>` with the given ID, and return
    /// a mutable reference to the wrapped view.
    ///
    /// [`IdView`]: views/struct.IdView.html
    pub fn find_id<V>(&mut self, id: &str) -> Option<views::ViewRef<V>>
    where
        V: View + Any,
    {
        self.call_on_id(id, views::IdView::<V>::get_mut)
    }

    /// Moves the focus to the view identified by `id`.
    ///
    /// Convenient method to call `focus` with a `view::Selector::Id`.
    pub fn focus_id(&mut self, id: &str) -> Result<(), ()> {
        self.focus(&view::Selector::Id(id))
    }

    /// Moves the focus to the view identified by `sel`.
    pub fn focus(&mut self, sel: &view::Selector) -> Result<(), ()> {
        self.screen_mut().focus_view(sel)
    }

    /// Adds a global callback.
    ///
    /// Will be triggered on the given key press when no view catches it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # extern crate cursive;
    /// # use cursive::*;
    /// # fn main() {
    /// let mut siv = Cursive::new();
    ///
    /// siv.add_global_callback('q', |s| s.quit());
    /// # }
    /// ```
    pub fn add_global_callback<F, E: Into<Event>>(&mut self, event: E, cb: F)
    where
        F: Fn(&mut Cursive) + 'static,
    {
        self.global_callbacks
            .insert(event.into(), Callback::from_fn(cb));
    }

    /// Add a layer to the current screen.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # extern crate cursive;
    /// # use cursive::*;
    /// # fn main() {
    /// let mut siv = Cursive::new();
    ///
    /// siv.add_layer(views::TextView::new("Hello world!"));
    /// # }
    /// ```
    pub fn add_layer<T: 'static + View>(&mut self, view: T) {
        self.screen_mut().add_layer(view);
    }

    /// Adds a new full-screen layer to the current screen.
    ///
    /// Fullscreen layers have no shadow.
    pub fn add_fullscreen_layer<T>(&mut self, view: T)
    where
        T: 'static + View,
    {
        self.screen_mut().add_fullscreen_layer(view);
    }

    /// Convenient method to remove a layer from the current screen.
    pub fn pop_layer(&mut self) {
        self.screen_mut().pop_layer();
        self.clear();
    }

    // Handles a key event when it was ignored by the current view
    fn on_event(&mut self, event: Event) {
        let cb = match self.global_callbacks.get(&event) {
            None => return,
            Some(cb) => cb.clone(),
        };
        // Not from a view, so no viewpath here
        cb(self);
    }

    /// Returns the size of the screen, in characters.
    pub fn screen_size(&self) -> Vec2 {
        let (x, y) = self.backend.screen_size();

        Vec2 {
            x: x as usize,
            y: y as usize,
        }
    }

    fn layout(&mut self) {
        let size = self.screen_size();
        self.screen_mut().layout(size);
    }

    fn draw(&mut self) {
        let sizes = self.screen().layer_sizes();
        if self.last_sizes != sizes {
            self.clear();
            self.last_sizes = sizes;
        }

        let printer =
            Printer::new(self.screen_size(), &self.theme, &self.backend);

        // Draw the currently active screen
        // If the menubar is active, nothing else can be.
        let offset = if self.menubar.autohide { 0 } else { 1 };
        // Draw the menubar?
        if self.menubar.visible() {
            let printer = printer.sub_printer(
                Vec2::zero(),
                printer.size,
                self.menubar.receive_events(),
            );
            self.menubar.draw(&printer);
        }

        let selected = self.menubar.receive_events();

        let printer =
            printer.sub_printer(Vec2::new(0, offset), printer.size, !selected);
        let id = self.active_screen;
        self.screens[id].draw(&printer);
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
    /// Calls [`step(&mut self)`] until [`quit(&mut self)`] is called.
    ///
    /// After this function returns, you can call
    /// it again and it will start a new loop.
    ///
    /// [`step(&mut self)`]: #method.step
    /// [`quit(&mut self)`]: #method.quit
    pub fn run(&mut self) {
        self.running = true;

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
    /// [`run(&mut self)`]: #method.run
    pub fn step(&mut self) {
        if let Ok(cb) = self.cb_source.try_recv() {
            cb(self);
        }

        // Do we need to redraw everytime?
        // Probably, actually.
        // TODO: Do we need to re-layout everytime?
        self.layout();

        // TODO: Do we need to redraw every view every time?
        // (Is this getting repetitive? :p)
        self.draw();
        self.backend.refresh();

        // Wait for next event.
        // (If set_fps was called, this returns -1 now and then)
        let event = self.backend.poll_event();
        if event == Event::Exit {
            self.quit();
        }

        if event == Event::WindowResize {
            self.clear();
        }

        // Event dispatch order:
        // * Focused element:
        //     * Menubar (if active)
        //     * Current screen (top layer)
        // * Global callbacks
        if self.menubar.receive_events() {
            self.menubar.on_event(event).process(self);
        } else {
            match self.screen_mut().on_event(event.clone()) {
                // If the event was ignored,
                // it is our turn to play with it.
                EventResult::Ignored => self.on_event(event),
                EventResult::Consumed(None) => (),
                EventResult::Consumed(Some(cb)) => cb(self),
            }
        }
    }

    /// Stops the event loop.
    pub fn quit(&mut self) {
        self.running = false;
    }
}

impl Drop for Cursive {
    fn drop(&mut self) {
        self.backend.finish();
    }
}
