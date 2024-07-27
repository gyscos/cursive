use crate::{
    direction::Direction,
    event::{Event, EventResult, Key},
    view::{View, ViewWrapper},
    With,
};

/// Adds circular focus to a wrapped view.
///
/// Wrap a view in `CircularFocus` to enable wrap-around focus
/// (when the focus exits this view, it will come back the other side).
///
/// It can be configured to wrap Tab (and Shift+Tab) keys, and/or Arrow keys.
pub struct CircularFocus<T: View> {
    view: T,
    wrap_tab: bool,
    wrap_up_down: bool,
    wrap_left_right: bool,
}

impl<T: View> CircularFocus<T> {
    /// Creates a new `CircularFocus` around the given view.
    ///
    /// Does not wrap anything by default,
    /// so you'll want to call one of the setters.
    pub fn new(view: T) -> Self {
        CircularFocus {
            view,
            wrap_tab: false,
            wrap_left_right: false,
            wrap_up_down: false,
        }
    }

    /// Returns `true` if Tab key cause focus to wrap around.
    pub fn wraps_tab(&self) -> bool {
        self.wrap_tab
    }

    /// Returns `true` if Arrow keys cause focus to wrap around.
    pub fn wraps_arrows(&self) -> bool {
        self.wrap_left_right && self.wrap_up_down
    }

    /// Return `true` if left/right keys cause focus to wrap around.
    pub fn wraps_left_right(&self) -> bool {
        self.wrap_left_right
    }

    /// Return `true` if up/down keys cause focus to wrap around.
    pub fn wraps_up_down(&self) -> bool {
        self.wrap_up_down
    }

    /// Make this view now wrap focus around when arrow keys are pressed.
    #[must_use]
    pub fn wrap_arrows(self) -> Self {
        self.with_wrap_arrows(true)
    }

    /// Make this view now wrap focus around when the up/down keys are pressed.
    #[must_use]
    pub fn wrap_up_down(self) -> Self {
        self.with_wrap_up_down(true)
    }

    /// Make this view now wrap focus around when the left/right keys are pressed.
    #[must_use]
    pub fn wrap_left_right(self) -> Self {
        self.with_wrap_left_right(true)
    }

    /// Make this view now wrap focus around when the Tab key is pressed.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_wrap_tab(self, wrap_tab: bool) -> Self {
        self.with(|s| s.set_wrap_tab(wrap_tab))
    }

    /// Make this view now wrap focus around when the Tab key is pressed.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn wrap_tab(self) -> Self {
        self.with_wrap_tab(true)
    }

    /// Make this view now wrap focus around when the left/right keys are pressed.
    #[must_use]
    pub fn with_wrap_left_right(self, wrap_left_right: bool) -> Self {
        self.with(|s| s.set_wrap_left_right(wrap_left_right))
    }

    /// Make this view now wrap focus around when the up/down keys are pressed.
    #[must_use]
    pub fn with_wrap_up_down(self, wrap_up_down: bool) -> Self {
        self.with(|s| s.set_wrap_up_down(wrap_up_down))
    }

    /// Make this view now wrap focus around when arrow keys are pressed.
    #[must_use]
    pub fn with_wrap_arrows(self, wrap_arrows: bool) -> Self {
        self.with(|s| s.set_wrap_arrows(wrap_arrows))
    }

    /// Make this view now wrap focus around when the Tab key is pressed.
    pub fn set_wrap_tab(&mut self, wrap_tab: bool) {
        self.wrap_tab = wrap_tab;
    }

    /// Make this view now wrap focus around when arrow keys are pressed.
    pub fn set_wrap_arrows(&mut self, wrap_arrows: bool) {
        self.wrap_left_right = wrap_arrows;
        self.wrap_up_down = wrap_arrows;
    }

    /// Make this view now wrap focus around when the up/down keys are pressed.
    pub fn set_wrap_up_down(&mut self, wrap_up_down: bool) {
        self.wrap_up_down = wrap_up_down;
    }

    /// Make this view now wrap focus around when the left/right keys are pressed.
    pub fn set_wrap_left_right(&mut self, wrap_left_right: bool) {
        self.wrap_left_right = wrap_left_right;
    }

    #[allow(unused)]
    fn set_wrap(&mut self, wrap_kind: WrapKind, wrap: bool) {
        match wrap_kind {
            WrapKind::Tab => self.set_wrap_tab(wrap),
            WrapKind::Arrows => self.set_wrap_arrows(wrap),
            WrapKind::LeftRight => self.set_wrap_left_right(wrap),
            WrapKind::UpDown => self.set_wrap_up_down(wrap),
        }
    }

    inner_getters!(self.view: T);
}

#[derive(Hash)]
#[allow(unused)]
enum WrapKind {
    Tab,
    Arrows,
    LeftRight,
    UpDown,
}

impl<T: View> ViewWrapper for CircularFocus<T> {
    wrap_impl!(self.view: T);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        match (self.view.on_event(event.clone()), event) {
            (EventResult::Ignored, Event::Key(Key::Tab)) if self.wrap_tab => {
                // Focus comes back!
                self.view
                    .take_focus(Direction::front())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Shift(Key::Tab)) if self.wrap_tab => {
                // Focus comes back!
                self.view
                    .take_focus(Direction::back())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Key(Key::Right)) if self.wrap_left_right => {
                // Focus comes back!
                self.view
                    .take_focus(Direction::left())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Key(Key::Left)) if self.wrap_left_right => {
                // Focus comes back!
                self.view
                    .take_focus(Direction::right())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Key(Key::Up)) if self.wrap_up_down => {
                // Focus comes back!
                self.view
                    .take_focus(Direction::down())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Key(Key::Down)) if self.wrap_up_down => {
                // Focus comes back!
                self.view
                    .take_focus(Direction::up())
                    .unwrap_or(EventResult::Ignored)
            }
            (other, _) => other,
        }
    }
}
/*

#[crate::blueprint(with = "circular_focus", CircularFocus::new)]
enum Blueprint {
    #[blueprint(
        set_wrap(wrap_kind, true),
        from=String,
    )]
    String(WrapKind),

    #[blueprint(
        foreach=set_wrap(wrap_kind, true),
        from=Array,
    )]
    Array(Vec<WrapKind>),

    #[blueprint(
        foreach=set_wrap,
        from=Object
    )]
    Object(HashMap<WrapKind, bool>),
}
*/

crate::manual_blueprint!(with circular_focus, |config, context| {
    use crate::builder::{Config, Error};

    // TODO: enable variable resolution across types
    // (for example `circular_focus: $focus` where $focus is an array.)
    // Especially for secondary template, where the variables are directly configs.
    //
    // Ex:
    // # circular_focus.yaml
    // View:
    //      view: $child
    //      with:
    //          - name: $name
    //          - circular_focus: $focus
    //
    // Maybe a method to "peek/resolve" a config value _as a config_? And then match that?
    // (Would return an error if the variable cannot be config'ed)
    // Or instead try resolving different types in the blueprint.
    fn parse_keyword(word: &str) -> Result<(bool, bool, bool), Error> {
        Ok(match word {
            "tab" => (true, false, false),
            "arrows" => (false, true, true),
            "left_right" => (false, false, true),
            "up_down" => (false, true, false),
            _ => {
                return Err(Error::InvalidConfig {
                    message: "Unrecognized circular focus style".into(),
                    config: word.into(),
                })
            }
        })
    }

    let (tab, up_down, left_right) = match config {
        Config::String(config) => parse_keyword(config)?,
        Config::Array(config) => {
            // Array: combine everything.
            let (mut tab, mut up_down, mut left_right) = (false, false, false);

            for config in config {
                let config: String = context.resolve(config)?;
                let (t, u, l) = parse_keyword(&config)?;
                tab |= t;
                up_down |= u;
                left_right |= l;
            }

            (tab, up_down, left_right)
        }
        Config::Object(config) => {
            let tab = context.resolve(&config["tab"])?;
            let mut left_right = context.resolve_or(&config["left_right"], false)?;
            let mut up_down = context.resolve_or(&config["up_down"], false)?;

            if let Some(arrows) = context.resolve(&config["arrows"])? {
                left_right = arrows;
                up_down = arrows;
            }

            (tab, up_down, left_right)
        }
        _ => (false, false, false),
    };

    Ok(move |view| {
        CircularFocus::new(view)
            .with_wrap_tab(tab)
            .with_wrap_left_right(left_right)
            .with_wrap_up_down(up_down)
    })
});
