use crate::{
    align::HAlign,
    direction::Direction,
    event::*,
    rect::Rect,
    style::PaletteStyle,
    utils::markup::StyledString,
    view::{CannotFocus, View},
    Cursive, Printer, Vec2,
};

/// Simple text label with a callback when `<Enter>` is pressed.
///
/// A button shows its content in a single line and has a fixed size.
///
/// # Examples
///
/// ```
/// use cursive_core::views::Button;
///
/// let quit_button = Button::new("Quit", |s| s.quit());
/// ```
pub struct Button {
    label: StyledString,
    callback: Callback,
    enabled: bool,
    last_size: Vec2,

    invalidated: bool,
}

impl Button {
    impl_enabled!(self.enabled);

    /// Creates a new button with the given content and callback.
    #[crate::callback_helpers]
    pub fn new<F, S>(label: S, cb: F) -> Self
    where
        F: 'static + Fn(&mut Cursive) + Send + Sync,
        S: Into<StyledString>,
    {
        let label = label.into();
        let label: StyledString =
            StyledString::concatenate([StyledString::plain("<"), label, StyledString::plain(">")]);

        Self::new_raw(label, cb)
    }

    /// Creates a new button without angle brackets.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::Button;
    ///
    /// let button = Button::new_raw("[ Quit ]", |s| s.quit());
    /// ```
    pub fn new_raw<F, S: Into<StyledString>>(label: S, cb: F) -> Self
    where
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        Button {
            label: label.into(),
            callback: Callback::from_fn(cb),
            enabled: true,
            last_size: Vec2::zero(),
            invalidated: true,
        }
    }

    /// Sets the function to be called when the button is pressed.
    ///
    /// Replaces the previous callback.
    pub fn set_callback<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive) + 'static + Send + Sync,
    {
        self.callback = Callback::from_fn(cb);
    }

    /// Returns the label for this button.
    ///
    /// Includes brackets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::views::Button;
    /// let button = Button::new("Quit", |s| s.quit());
    /// assert_eq!(button.label(), "<Quit>");
    /// ```
    pub fn label(&self) -> &str {
        self.label.source()
    }

    /// Sets the label to the given value.
    ///
    /// This will include brackets.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::Button;
    ///
    /// let mut button = Button::new("Quit", |s| s.quit());
    /// button.set_label("Escape");
    /// ```
    pub fn set_label<S>(&mut self, label: S)
    where
        S: Into<String>,
    {
        self.set_label_raw(format!("<{}>", label.into()));
    }

    /// Sets the label exactly to the given value.
    ///
    /// This will not include brackets.
    pub fn set_label_raw<S>(&mut self, label: S)
    where
        S: Into<StyledString>,
    {
        self.label = label.into();
        self.invalidate();
    }

    fn req_size(&self) -> Vec2 {
        Vec2::new(self.label.width(), 1)
    }

    fn invalidate(&mut self) {
        self.invalidated = true;
    }
}

impl View for Button {
    fn draw(&self, printer: &Printer) {
        if printer.size.x == 0 {
            return;
        }

        let style = if !(self.enabled && printer.enabled) {
            // Disabled button goes blue
            PaletteStyle::Secondary
        } else if printer.focused {
            // Selected button is highlighted
            PaletteStyle::Highlight
        } else {
            // Looks like regular text if not selected
            PaletteStyle::Primary
        };

        let offset = HAlign::Center.get_offset(self.label.width(), printer.size.x);

        // eprintln!("Button style: {style:?}");
        printer.with_style(style, |printer| {
            // TODO: do we want to "fill" the button highlight color to the full given size?
            // printer.print_hline((0, 0), offset, " ");
            printer.print_styled((offset, 0), &self.label);
            // let end = offset + self.label.width();
            // printer.print_hline(
            //     (end, 0),
            //     printer.size.x.saturating_sub(end),
            //     " ",
            // );
        });
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;
        self.invalidated = false;
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        // Meh. Fixed size we are.
        self.req_size()
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }

        // eprintln!("{:?}", event);
        // eprintln!("{:?}", self.req_size());
        let width = self.label.width();
        let self_offset = HAlign::Center.get_offset(width, self.last_size.x);
        match event {
            Event::Key(Key::Enter) => EventResult::Consumed(Some(self.callback.clone())),
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset + (self_offset, 0), self.req_size()) => {
                EventResult::Consumed(Some(self.callback.clone()))
            }
            _ => EventResult::Ignored,
        }
    }

    fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
        self.enabled.then(EventResult::consumed).ok_or(CannotFocus)
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        let width = self.label.width();
        let offset = HAlign::Center.get_offset(width, view_size.x);

        Rect::from_size((offset, 0), (width, 1))
    }

    fn needs_relayout(&self) -> bool {
        self.invalidated
    }
}

// Here we use the `new_with_cb` method generated by `callback_helpers`.
// Note that the `Blueprint` type will not actually exist.
#[crate::blueprint(Button::new_with_cb(label, callback))]
struct Blueprint {
    // Fields mentioned in the initializer will be used there
    label: StyledString,

    // Callback types can be omitted
    callback: _,

    // Other fields will use `set_$name`
    enabled: Option<bool>,
}

// The above blueprint will expand to this code:
/*
crate::manual_blueprint!(Button, |config, context| {
    Ok({
        let label: StyledString = context.resolve(&config["label"])?;
        let callback: _ = context.resolve(&config["callback"])?;
        let mut button = Button::new_with_cb(label, callback);

        let enabled: Option<bool> = context.resolve(&config["enabled"])?;
        if let Some(enabled) = enabled {
            button.set_enabled(enabled);
        }

        button
    })
});
*/
