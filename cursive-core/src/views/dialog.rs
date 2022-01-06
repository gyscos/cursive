use crate::{
    align::*,
    direction::{Absolute, Direction, Relative},
    event::{AnyCb, Event, EventResult, Key},
    rect::Rect,
    theme::ColorStyle,
    utils::markup::StyledString,
    view::{
        CannotFocus, IntoBoxedView, Margins, Selector, View, ViewNotFound,
    },
    views::{BoxedView, Button, DummyView, LastSizeView, TextView},
    Cursive, Printer, Vec2, With,
};
use std::cell::Cell;
use std::cmp::{max, min};
use unicode_width::UnicodeWidthStr;

/// Identifies currently focused element in [`Dialog`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DialogFocus {
    /// Content element focused
    Content,
    /// One of buttons focused
    Button(usize),
}

struct ChildButton {
    button: LastSizeView<Button>,
    offset: Cell<Vec2>,
}

impl ChildButton {
    pub fn new<F, S: Into<String>>(label: S, cb: F) -> Self
    where
        F: 'static + Fn(&mut Cursive),
    {
        ChildButton {
            button: LastSizeView::new(Button::new(label, cb)),
            offset: Cell::new(Vec2::zero()),
        }
    }
}

/// Popup-like view with a main content, and optional buttons under it.
///
/// # Examples
///
/// ```
/// # use cursive_core::views::{Dialog,TextView};
/// let dialog =
///     Dialog::around(TextView::new("Hello!")).button("Ok", |s| s.quit());
/// ```
pub struct Dialog {
    // Possibly empty title.
    title: String,

    // Where to put the title position
    title_position: HAlign,

    // The actual inner view.
    content: LastSizeView<BoxedView>,

    // Optional list of buttons under the main view.
    // Include the top-left corner.
    buttons: Vec<ChildButton>,

    // Padding around the inner view.
    padding: Margins,

    // Borders around everything.
    borders: Margins,

    // The current element in focus
    focus: DialogFocus,

    // How to align the buttons under the view.
    align: Align,

    // `true` when we needs to relayout
    invalidated: bool,
}

new_default!(Dialog);

impl Dialog {
    /// Creates a new `Dialog` with empty content.
    ///
    /// You should probably call `content()` next.
    pub fn new() -> Self {
        Self::around(DummyView)
    }

    /// Creates a new `Dialog` with the given content.
    pub fn around<V: IntoBoxedView>(view: V) -> Self {
        Dialog {
            content: LastSizeView::new(BoxedView::boxed(view)),
            buttons: Vec::new(),
            title: String::new(),
            title_position: HAlign::Center,
            focus: DialogFocus::Content,
            padding: Margins::lr(1, 1),
            borders: Margins::lrtb(1, 1, 1, 1),
            align: Align::top_right(),
            invalidated: true,
        }
    }

    /// Sets the content for this dialog.
    ///
    /// Chainable variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::{Dialog, TextView};
    ///
    /// let dialog = Dialog::new()
    ///     .content(TextView::new("Hello!"))
    ///     .button("Quit", |s| s.quit());
    /// ```
    pub fn content<V: IntoBoxedView>(self, view: V) -> Self {
        self.with(|s| {
            s.set_content(view);
        })
    }

    /// Gets the content of this dialog.
    ///
    /// ```
    /// use cursive_core::views::{Dialog, TextView};
    /// let dialog = Dialog::around(TextView::new("Hello!"));
    /// let text_view: &TextView =
    ///     dialog.get_content().downcast_ref::<TextView>().unwrap();
    /// assert_eq!(text_view.get_content().source(), "Hello!");
    /// ```
    pub fn get_content(&self) -> &dyn View {
        &*self.content.view
    }

    /// Gets mutable access to the content.
    pub fn get_content_mut(&mut self) -> &mut dyn View {
        self.invalidate();
        &mut *self.content.view
    }

    /// Consumes `self` and returns the boxed content view.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::view::View;
    /// use cursive_core::views::{Dialog, TextView};
    ///
    /// let dialog = Dialog::around(TextView::new("abc"));
    ///
    /// let content: Box<dyn View> = dialog.into_content();
    /// assert!(content.is::<TextView>());
    ///
    /// let content: Box<TextView> = content.downcast().ok().unwrap();
    /// assert_eq!(content.get_content().source(), "abc");
    /// ```
    pub fn into_content(self) -> Box<dyn View> {
        self.content.view.unwrap()
    }

    /// Sets the content for this dialog.
    ///
    /// Previous content will be returned.
    pub fn set_content<V: IntoBoxedView>(&mut self, view: V) -> Box<dyn View> {
        self.invalidate();
        std::mem::replace(
            &mut self.content,
            LastSizeView::new(BoxedView::boxed(view)),
        )
        .view
        .unwrap()
    }

    /// Convenient method to create a dialog with a simple text content.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::Dialog;
    ///
    /// let dialog = Dialog::text("Hello!").button("Quit", |s| s.quit());
    /// ```
    pub fn text<S: Into<StyledString>>(text: S) -> Self {
        Self::around(TextView::new(text))
    }

    /// Convenient method to create an infobox.
    ///
    /// It will contain the given text and a `Ok` dismiss button.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::Dialog;
    ///
    /// let dialog = Dialog::info("Some very important information!");
    /// ```
    pub fn info<S: Into<StyledString>>(text: S) -> Self {
        Dialog::text(text).dismiss_button("Ok")
    }

    /// Adds a button to the dialog with the given label and callback.
    ///
    /// Consumes and returns self for easy chaining.
    pub fn button<F, S: Into<String>>(self, label: S, cb: F) -> Self
    where
        F: 'static + Fn(&mut Cursive),
    {
        self.with(|s| s.add_button(label, cb))
    }

    /// Adds a button to the dialog with the given label and callback.
    pub fn add_button<F, S: Into<String>>(&mut self, label: S, cb: F)
    where
        F: 'static + Fn(&mut Cursive),
    {
        self.buttons.push(ChildButton::new(label, cb));
        self.invalidate();
    }

    /// Returns the number of buttons on this dialog.
    pub fn buttons_len(&self) -> usize {
        self.buttons.len()
    }

    /// Removes any button from `self`.
    pub fn clear_buttons(&mut self) -> EventResult {
        self.buttons.clear();
        self.invalidate();
        if self.focus != DialogFocus::Content {
            self.focus = DialogFocus::Content;
            self.content
                .take_focus(Direction::none())
                .unwrap_or(EventResult::Ignored)
        } else {
            EventResult::Ignored
        }
    }

    /// Removes a button from this dialog.
    ///
    /// # Panics
    ///
    /// Panics if `i >= self.buttons_len()`.
    pub fn remove_button(&mut self, i: usize) -> EventResult {
        self.buttons.remove(i);
        self.invalidate();
        // Fix focus?
        match (self.buttons.len(), self.focus) {
            (0, ref mut focus) => {
                *focus = DialogFocus::Content;
                return self
                    .content
                    .take_focus(Direction::none())
                    .unwrap_or(EventResult::Ignored);
            }
            (n, DialogFocus::Button(ref mut i)) => {
                *i = usize::min(*i, n - 1);
            }
            _ => (),
        }
        EventResult::Ignored
    }

    /// Sets the horizontal alignment for the buttons, if any.
    ///
    /// Only works if the buttons are as a row at the bottom of the dialog.
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
    }

    /// Gets the horizontal alignment for the buttons.
    pub fn get_h_align(&self) -> HAlign {
        self.align.h
    }

    /*
     * Commented out because currently un-implemented.
     *
    /// Sets the vertical alignment for the buttons, if any.
    ///
    /// Only works if the buttons are as a column to the right of the dialog.
    pub fn v_align(mut self, v: VAlign) -> Self {
        self.align.v = v;

        self
    }
    */

    /// Shortcut method to add a button that will dismiss the dialog.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::Dialog;
    ///
    /// let dialog = Dialog::text("Hello!").dismiss_button("Close");
    /// ```
    pub fn dismiss_button<S: Into<String>>(self, label: S) -> Self {
        self.button(label, |s| {
            s.pop_layer();
        })
    }

    /// Sets the title of the dialog.
    ///
    /// If not empty, it will be visible at the top.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive_core::views::Dialog;
    ///
    /// let dialog = Dialog::info("Some info").title("Read me!");
    /// ```
    pub fn title<S: Into<String>>(self, label: S) -> Self {
        self.with(|s| s.set_title(label))
    }

    /// Sets the title of the dialog.
    pub fn set_title<S: Into<String>>(&mut self, label: S) {
        self.title = label.into();
        self.invalidate();
    }

    /// Get the title of the dialog.
    pub fn get_title(&self) -> &str {
        &self.title
    }

    /// Sets the horizontal position of the title in the dialog.
    /// The default position is `HAlign::Center`
    pub fn title_position(self, align: HAlign) -> Self {
        self.with(|s| s.set_title_position(align))
    }

    /// Sets the horizontal position of the title in the dialog.
    /// The default position is `HAlign::Center`
    pub fn set_title_position(&mut self, align: HAlign) {
        self.title_position = align;
    }

    /// Gets the alignment of the title
    pub fn get_title_position(&self) -> HAlign {
        self.title_position
    }

    /// Sets the padding in the dialog (around content and buttons).
    ///
    /// # Examples
    /// ```
    /// use cursive_core::views::Dialog;
    /// use cursive_core::view::Margins;
    ///
    /// let dialog = Dialog::info("Hello!")
    ///         .padding(Margins::lrtb(1, 1, 0, 0)); // (Left, Right, Top, Bottom)
    /// ```
    pub fn padding(self, padding: Margins) -> Self {
        self.with(|s| s.set_padding(padding))
    }

    /// Gets the padding in the dialog (around content and buttons).
    pub fn get_padding(&self) -> Margins {
        self.padding
    }

    /// Sets the padding in the dialog.
    ///
    /// Takes Left, Right, Top, Bottom fields.
    pub fn padding_lrtb(
        self,
        left: usize,
        right: usize,
        top: usize,
        bottom: usize,
    ) -> Self {
        self.padding(Margins::lrtb(left, right, top, bottom))
    }

    /// Sets the padding in the dialog (around content and buttons).
    ///
    /// Chainable variant.
    pub fn set_padding(&mut self, padding: Margins) {
        self.padding = padding;
    }

    /// Sets the top padding in the dialog (under the title).
    pub fn padding_top(self, padding: usize) -> Self {
        self.with(|s| s.set_padding_top(padding))
    }

    /// Sets the top padding in the dialog (under the title).
    pub fn set_padding_top(&mut self, padding: usize) {
        self.padding.top = padding;
    }

    /// Sets the bottom padding in the dialog (under buttons).
    pub fn padding_bottom(self, padding: usize) -> Self {
        self.with(|s| s.set_padding_bottom(padding))
    }

    /// Sets the bottom padding in the dialog (under buttons).
    pub fn set_padding_bottom(&mut self, padding: usize) {
        self.padding.bottom = padding;
    }

    /// Sets the left padding in the dialog.
    pub fn padding_left(self, padding: usize) -> Self {
        self.with(|s| s.set_padding_left(padding))
    }

    /// Sets the left padding in the dialog.
    pub fn set_padding_left(&mut self, padding: usize) {
        self.padding.left = padding;
    }

    /// Sets the right padding in the dialog.
    pub fn padding_right(self, padding: usize) -> Self {
        self.with(|s| s.set_padding_right(padding))
    }

    /// Sets the right padding in the dialog.
    pub fn set_padding_right(&mut self, padding: usize) {
        self.padding.right = padding;
    }

    /// Iterate the buttons of this dialog.
    pub fn buttons(&self) -> impl Iterator<Item = &Button> {
        self.buttons.iter().map(|b| &b.button.view)
    }

    /// Mutably iterate the buttons of this dialog.
    pub fn buttons_mut(&mut self) -> impl Iterator<Item = &mut Button> {
        self.invalidate();
        self.buttons.iter_mut().map(|b| &mut b.button.view)
    }

    /// Returns currently focused element
    pub fn focus(&self) -> DialogFocus {
        self.focus
    }

    /// Change the current focus of the dialog.
    ///
    /// Please be considerate of the context from which focus is being stolen
    /// when programmatically moving focus. For example, moving focus to a
    /// button when a user is typing something into an `EditView` would cause
    /// them to accidentally activate the button.
    ///
    /// The given dialog focus will be clamped to a valid range. For example,
    /// attempting to focus a button that no longer exists will instead focus
    /// one that does (or the content, if no buttons exist).
    pub fn set_focus(&mut self, new_focus: DialogFocus) -> EventResult {
        let mut result = EventResult::Ignored;

        self.focus = match new_focus {
            DialogFocus::Content => DialogFocus::Content,
            DialogFocus::Button(_) if self.buttons.is_empty() => {
                DialogFocus::Content
            }
            DialogFocus::Button(c) => {
                if self.focus == DialogFocus::Content {
                    result = self.content.on_event(Event::FocusLost);
                }
                DialogFocus::Button(min(c, self.buttons.len() - 1))
            }
        };

        result
    }

    // Private methods

    // An event is received while the content is in focus
    fn on_event_content(&mut self, event: Event) -> EventResult {
        match self.content.on_event(
            event.relativized((self.padding + self.borders).top_left()),
        ) {
            EventResult::Ignored => {
                if self.buttons.is_empty() {
                    EventResult::Ignored
                } else {
                    match event {
                        Event::Key(Key::Down) | Event::Key(Key::Tab) => {
                            // Default to leftmost button when going down.
                            self.focus = DialogFocus::Button(0);

                            // Return the content's FocusLost trigger, but also make sure it is
                            // consumed.
                            EventResult::Consumed(None)
                                .and(self.content.on_event(Event::FocusLost))
                        }
                        _ => EventResult::Ignored,
                    }
                }
            }
            res => res,
        }
    }

    // An event is received while a button is in focus
    fn on_event_button(
        &mut self,
        event: Event,
        button_id: usize,
    ) -> EventResult {
        let result = {
            let button = &mut self.buttons[button_id];
            button
                .button
                .on_event(event.relativized(button.offset.get()))
        };
        match result {
            EventResult::Ignored => {
                match event {
                    // Up goes back to the content
                    Event::Key(Key::Up) => {
                        if let Ok(res) =
                            self.content.take_focus(Direction::down())
                        {
                            self.focus = DialogFocus::Content;
                            res
                        } else {
                            EventResult::Ignored
                        }
                    }
                    Event::Shift(Key::Tab)
                        if self.focus == DialogFocus::Button(0) =>
                    {
                        // If we're at the first button, jump back to the content.
                        if let Ok(res) =
                            self.content.take_focus(Direction::back())
                        {
                            self.focus = DialogFocus::Content;
                            res
                        } else {
                            EventResult::Ignored
                        }
                    }
                    Event::Shift(Key::Tab) => {
                        // Otherwise, jump to the previous button.
                        if let DialogFocus::Button(ref mut i) = self.focus {
                            // This should always be the case.
                            *i -= 1;
                        }
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Tab)
                        if self.focus
                            == DialogFocus::Button(
                                self.buttons.len().saturating_sub(1),
                            ) =>
                    {
                        // End of the line
                        EventResult::Ignored
                    }
                    Event::Key(Key::Tab) => {
                        // Otherwise, jump to the next button.
                        if let DialogFocus::Button(ref mut i) = self.focus {
                            // This should always be the case.
                            *i += 1;
                        }
                        EventResult::Consumed(None)
                    }
                    // Left and Right move to other buttons
                    Event::Key(Key::Right)
                        if button_id + 1 < self.buttons.len() =>
                    {
                        self.focus = DialogFocus::Button(button_id + 1);
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Left) if button_id > 0 => {
                        self.focus = DialogFocus::Button(button_id - 1);
                        EventResult::Consumed(None)
                    }
                    _ => EventResult::Ignored,
                }
            }
            res => res,
        }
    }

    fn draw_buttons(&self, printer: &Printer) -> Option<usize> {
        let mut buttons_height = 0;
        // Current horizontal position of the next button we'll draw.

        // Sum of the sizes + len-1 for margins
        let width = self
            .buttons
            .iter()
            .map(|button| button.button.size.x)
            .sum::<usize>()
            + self.buttons.len().saturating_sub(1);
        let overhead = self.padding + self.borders;
        if printer.size.x < overhead.horizontal() {
            return None;
        }
        let mut offset = overhead.left
            + self
                .align
                .h
                .get_offset(width, printer.size.x - overhead.horizontal());

        let overhead_bottom = self.padding.bottom + self.borders.bottom + 1;

        let y = match printer.size.y.checked_sub(overhead_bottom) {
            Some(y) => y,
            None => return None,
        };

        for (i, button) in self.buttons.iter().enumerate() {
            let size = button.button.size;
            // Add some special effect to the focused button
            let position = Vec2::new(offset, y);
            button.offset.set(position);
            button.button.draw(
                &printer
                    .offset(position)
                    .cropped(size)
                    .focused(self.focus == DialogFocus::Button(i)),
            );
            // Keep 1 blank between two buttons
            offset += size.x + 1;
            // Also keep 1 blank above the buttons
            buttons_height = max(buttons_height, size.y + 1);
        }

        Some(buttons_height)
    }

    fn draw_content(&self, printer: &Printer, buttons_height: usize) {
        // What do we have left?
        let taken = Vec2::new(0, buttons_height)
            + self.borders.combined()
            + self.padding.combined();

        let inner_size = match printer.size.checked_sub(taken) {
            Some(s) => s,
            None => return,
        };

        self.content.draw(
            &printer
                .offset(self.borders.top_left() + self.padding.top_left())
                .cropped(inner_size)
                .focused(self.focus == DialogFocus::Content),
        );
    }

    fn draw_title(&self, printer: &Printer) {
        if !self.title.is_empty() {
            let len = self.title.width();
            let spacing = 3; //minimum distance to borders
            let spacing_both_ends = 2 * spacing;
            if len + spacing_both_ends > printer.size.x {
                return;
            }
            let x = spacing
                + self
                    .title_position
                    .get_offset(len, printer.size.x - spacing_both_ends);
            printer.with_high_border(false, |printer| {
                printer.print((x - 2, 0), "┤ ");
                printer.print((x + len, 0), " ├");
            });

            printer.with_color(ColorStyle::title_primary(), |p| {
                p.print((x, 0), &self.title)
            });
        }
    }

    fn check_focus_grab(&mut self, event: &Event) -> Option<EventResult> {
        if let Event::Mouse {
            offset,
            position,
            event,
        } = *event
        {
            if !event.grabs_focus() {
                return None;
            }

            let position = match position.checked_sub(offset) {
                None => return None,
                Some(pos) => pos,
            };

            // eprintln!("Rel pos: {:?}", position);

            // Now that we have a relative position, checks for buttons?
            if let Some(i) = self.buttons.iter().position(|btn| {
                // If position fits there...
                position.fits_in_rect(btn.offset.get(), btn.button.size)
            }) {
                return Some(self.set_focus(DialogFocus::Button(i)));
            } else if position.fits_in_rect(
                (self.padding + self.borders).top_left(),
                self.content.size,
            ) {
                if let Ok(res) = self.content.take_focus(Direction::none()) {
                    // Or did we click the content?
                    self.focus = DialogFocus::Content;
                    return Some(res);
                }
            }
        }
        None
    }

    fn invalidate(&mut self) {
        self.invalidated = true;
    }
}

impl View for Dialog {
    fn draw(&self, printer: &Printer) {
        // This will be the buttons_height used by the buttons.
        let buttons_height = match self.draw_buttons(printer) {
            Some(height) => height,
            None => return,
        };

        self.draw_content(printer, buttons_height);

        // Print the borders
        printer.print_box(Vec2::new(0, 0), printer.size, false);

        self.draw_title(printer);
    }

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        // Padding and borders are not available for kids.
        let nomans_land = self.padding.combined() + self.borders.combined();

        // Buttons are not flexible, so their size doesn't depend on ours.
        let mut buttons_size = Vec2::new(0, 0);

        // Start with the inter-button space.
        buttons_size.x += self.buttons.len().saturating_sub(1);

        for button in &mut self.buttons {
            let s = button.button.view.required_size(req);
            buttons_size.x += s.x;
            buttons_size.y = max(buttons_size.y, s.y + 1);
        }

        // We also remove one row for the buttons.
        let taken = nomans_land + Vec2::new(0, buttons_size.y);

        let content_req = match req.checked_sub(taken) {
            Some(r) => r,
            // Bad!!
            None => return taken,
        };

        let content_size = self.content.required_size(content_req);

        // On the Y axis, we add buttons and content.
        // On the X axis, we take the max.
        let mut inner_size = Vec2::new(
            max(content_size.x, buttons_size.x),
            content_size.y + buttons_size.y,
        ) + self.padding.combined()
            + self.borders.combined();

        if !self.title.is_empty() {
            // If we have a title, we have to fit it too!
            inner_size.x = max(inner_size.x, self.title.width() + 6);
        }

        inner_size
    }

    fn layout(&mut self, mut size: Vec2) {
        // Padding and borders are taken, sorry.
        // TODO: handle border-less themes?
        let taken = self.borders.combined() + self.padding.combined();
        size = size.saturating_sub(taken);

        // Buttons are kings, we give them everything they want.
        let mut buttons_height = 0;
        for button in self.buttons.iter_mut().rev() {
            let size = button.button.required_size(size);
            buttons_height = max(buttons_height, size.y + 1);
            button.button.layout(size);
        }

        // Poor content will have to make do with what's left.
        if buttons_height > size.y {
            buttons_height = size.y;
        }

        self.content
            .layout(size.saturating_sub((0, buttons_height)));

        self.invalidated = false;
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // First: some mouse events can instantly change the focus.
        let res = self
            .check_focus_grab(&event)
            .unwrap_or(EventResult::Ignored);

        res.and(match self.focus {
            // If we are on the content, we can only go down.
            // TODO: Careful if/when we add buttons elsewhere on the dialog!
            DialogFocus::Content => self.on_event_content(event),
            // If we are on a button, we have more choice
            DialogFocus::Button(i) => self.on_event_button(event, i),
        })
    }

    fn take_focus(
        &mut self,
        source: Direction,
    ) -> Result<EventResult, CannotFocus> {
        // TODO: This may depend on button position relative to the content?
        //
        match source {
            Direction::Abs(Absolute::None) => {
                // Only reject focus if no buttons and no focus-taking content.
                // Also fix focus if we're focusing the wrong thing.
                match (self.focus, !self.buttons.is_empty()) {
                    (DialogFocus::Button(_), true) => {
                        // Focus stays on the button.
                        Ok(EventResult::Consumed(None))
                    }
                    (DialogFocus::Button(_), false) => {
                        let res = self.content.take_focus(source);
                        if res.is_ok() {
                            self.focus = DialogFocus::Content;
                        }
                        res
                    }
                    (DialogFocus::Content, false) => {
                        self.content.take_focus(source)
                    }
                    (DialogFocus::Content, true) => {
                        // Content had focus, but now refuses to take it again. So it loses it.
                        match self.content.take_focus(source) {
                            Ok(res) => Ok(res),
                            Err(CannotFocus) => {
                                self.focus = DialogFocus::Button(0);
                                Ok(self
                                    .content
                                    .on_event(Event::FocusLost)
                                    .and(EventResult::consumed()))
                            }
                        }
                    }
                }
            }
            Direction::Rel(Relative::Front)
            | Direction::Abs(Absolute::Left)
            | Direction::Abs(Absolute::Up) => {
                // Forward focus: content, then buttons
                if let Ok(res) = self.content.take_focus(source) {
                    self.focus = DialogFocus::Content;
                    Ok(res)
                } else if self.buttons.is_empty() {
                    Err(CannotFocus)
                } else {
                    let mut result = EventResult::consumed();
                    if self.focus == DialogFocus::Content {
                        // The content had focus, but now refuses to take it.
                        result = result
                            .and(self.content.on_event(Event::FocusLost));
                    }
                    self.focus = DialogFocus::Button(0);
                    Ok(result)
                }
            }
            Direction::Rel(Relative::Back)
            | Direction::Abs(Absolute::Right)
            | Direction::Abs(Absolute::Down) => {
                // Back focus: first buttons, then content
                if !self.buttons.is_empty() {
                    let mut result = EventResult::consumed();
                    if self.focus == DialogFocus::Content {
                        result = result
                            .and(self.content.on_event(Event::FocusLost));
                    }
                    self.focus = DialogFocus::Button(self.buttons.len() - 1);
                    Ok(result)
                } else if let Ok(res) = self.content.take_focus(source) {
                    self.focus = DialogFocus::Content;
                    Ok(res)
                } else {
                    Err(CannotFocus)
                }
            }
        }
    }

    fn call_on_any<'a>(
        &mut self,
        selector: &Selector<'_>,
        callback: AnyCb<'a>,
    ) {
        self.content.call_on_any(selector, callback);
    }

    fn focus_view(
        &mut self,
        selector: &Selector<'_>,
    ) -> Result<EventResult, ViewNotFound> {
        self.content.focus_view(selector)
    }

    fn important_area(&self, _: Vec2) -> Rect {
        // Only the content is important.
        // TODO: if a button is focused, return the button position instead.
        self.content.important_area(self.content.size)
            + self.borders.top_left()
            + self.padding.top_left()
    }

    fn needs_relayout(&self) -> bool {
        self.invalidated || self.content.needs_relayout()
    }
}
