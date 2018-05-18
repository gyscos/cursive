use align::*;
use direction::Direction;
use event::{AnyCb, Event, EventResult, Key};
use rect::Rect;
use std::cell::Cell;
use std::cmp::max;
use theme::ColorStyle;
use unicode_width::UnicodeWidthStr;
use vec::Vec2;
use view::{Margins, Selector, View};
use views::{Button, DummyView, SizedView, TextView, ViewBox};
use Cursive;
use Printer;
use With;

/// Identifies currently focused element in [`Dialog`].
///
/// [`Dialog`]: struct.Dialog.html
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogFocus {
    /// Content element focused
    Content,
    /// One of buttons focused
    Button(usize),
}

struct ChildButton {
    button: SizedView<Button>,
    offset: Cell<Vec2>,
}

impl ChildButton {
    pub fn new<F, S: Into<String>>(label: S, cb: F) -> Self
    where
        F: 'static + Fn(&mut Cursive),
    {
        ChildButton {
            button: SizedView::new(Button::new(label, cb)),
            offset: Cell::new(Vec2::zero()),
        }
    }
}

/// Popup-like view with a main content, and optional buttons under it.
///
/// # Examples
///
/// ```
/// # use cursive::views::{Dialog,TextView};
/// let dialog = Dialog::around(TextView::new("Hello!"))
///                     .button("Ok", |s| s.quit());
/// ```
pub struct Dialog {
    // Possibly empty title.
    title: String,

    // Where to put the title position
    title_position: HAlign,

    // The actual inner view.
    content: SizedView<ViewBox>,

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
    pub fn around<V: View + 'static>(view: V) -> Self {
        Dialog {
            content: SizedView::new(ViewBox::boxed(view)),
            buttons: Vec::new(),
            title: String::new(),
            title_position: HAlign::Center,
            focus: DialogFocus::Content,
            padding: Margins::new(1, 1, 0, 0),
            borders: Margins::new(1, 1, 1, 1),
            align: Align::top_right(),
        }
    }

    /// Sets the content for this dialog.
    ///
    /// Chainable variant.
    pub fn content<V: View + 'static>(self, view: V) -> Self {
        self.with(|s| s.set_content(view))
    }

    /// Gets the content of this dialog.
    ///
    /// ```
    /// use cursive::views::{Dialog, TextView};
    /// let dialog = Dialog::around(TextView::new("Hello!"));
    /// let text_view: &TextView = dialog
    ///     .get_content()
    ///     .as_any()
    ///     .downcast_ref::<TextView>()
    ///     .unwrap();
    /// assert_eq!(text_view.get_content().source(), "Hello!");
    /// ```
    pub fn get_content(&self) -> &View {
        &*self.content.view
    }

    /// Gets mutable access to the content.
    pub fn get_content_mut(&mut self) -> &mut View {
        &mut *self.content.view
    }

    /// Sets the content for this dialog.
    ///
    /// Previous content will be dropped.
    pub fn set_content<V: View + 'static>(&mut self, view: V) {
        self.content = SizedView::new(ViewBox::boxed(view));
    }

    /// Convenient method to create a dialog with a simple text content.
    pub fn text<S: Into<String>>(text: S) -> Self {
        Self::around(TextView::new(text))
    }

    /// Convenient method to create an infobox.
    ///
    /// It will contain the given text and a `Ok` dismiss button.
    pub fn info<S: Into<String>>(text: S) -> Self {
        Dialog::text(text).dismiss_button("Ok")
    }

    /// Adds a button to the dialog with the given label and callback.
    ///
    /// Consumes and returns self for easy chaining.
    pub fn button<F, S: Into<String>>(mut self, label: S, cb: F) -> Self
    where
        F: 'static + Fn(&mut Cursive),
    {
        self.buttons.push(ChildButton::new(label, cb));

        self
    }

    /// Sets the horizontal alignment for the buttons, if any.
    ///
    /// Only works if the buttons are as a row at the bottom of the dialog.
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
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
    pub fn dismiss_button<S: Into<String>>(self, label: S) -> Self {
        self.button(label, |s| {
            s.pop_layer();
        })
    }

    /// Sets the title of the dialog.
    ///
    /// If not empty, it will be visible at the top.
    pub fn title<S: Into<String>>(self, label: S) -> Self {
        self.with(|s| s.set_title(label))
    }

    /// Sets the title of the dialog.
    pub fn set_title<S: Into<String>>(&mut self, label: S) {
        self.title = label.into();
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

    /// Sets the padding in the dialog (around content and buttons).
    pub fn padding<T: Into<Margins>>(mut self, padding: T) -> Self {
        self.padding = padding.into();

        self
    }

    /// Sets the top padding in the dialog (under the title).
    pub fn padding_top(mut self, padding: usize) -> Self {
        self.padding.top = padding;
        self
    }

    /// Sets the bottom padding in the dialog (under buttons).
    pub fn padding_bottom(mut self, padding: usize) -> Self {
        self.padding.bottom = padding;
        self
    }

    /// Sets the left padding in the dialog.
    pub fn padding_left(mut self, padding: usize) -> Self {
        self.padding.left = padding;
        self
    }

    /// Sets the right padding in the dialog.
    pub fn padding_right(mut self, padding: usize) -> Self {
        self.padding.right = padding;
        self
    }

    /// Returns an iterator on this buttons for this dialog.
    pub fn buttons_mut<'a>(
        &'a mut self,
    ) -> Box<'a + Iterator<Item = &'a mut Button>> {
        Box::new(self.buttons.iter_mut().map(|b| &mut b.button.view))
    }

    /// Returns currently focused element
    pub fn focus(&self) -> DialogFocus {
        self.focus
    }

    // Private methods

    // An event is received while the content is in focus
    fn on_event_content(&mut self, event: Event) -> EventResult {
        match self.content.on_event(
            event.relativized((self.padding + self.borders).top_left()),
        ) {
            EventResult::Ignored if !self.buttons.is_empty() => {
                match event {
                    Event::Key(Key::Down)
                    | Event::Key(Key::Tab)
                    | Event::Shift(Key::Tab) => {
                        // Default to leftmost button when going down.
                        self.focus = DialogFocus::Button(0);
                        EventResult::Consumed(None)
                    }
                    _ => EventResult::Ignored,
                }
            }
            res => res,
        }
    }

    // An event is received while a button is in focus
    fn on_event_button(
        &mut self, event: Event, button_id: usize,
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
                        if self.content.take_focus(Direction::down()) {
                            self.focus = DialogFocus::Content;
                            EventResult::Consumed(None)
                        } else {
                            EventResult::Ignored
                        }
                    }
                    Event::Shift(Key::Tab) => {
                        if self.content.take_focus(Direction::back()) {
                            self.focus = DialogFocus::Content;
                            EventResult::Consumed(None)
                        } else {
                            EventResult::Ignored
                        }
                    }
                    Event::Key(Key::Tab) => {
                        if self.content.take_focus(Direction::front()) {
                            self.focus = DialogFocus::Content;
                            EventResult::Consumed(None)
                        } else {
                            EventResult::Ignored
                        }
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
        let width = self.buttons
            .iter()
            .map(|button| button.button.size.x)
            .sum::<usize>()
            + self.buttons.len().saturating_sub(1);
        let overhead = self.padding + self.borders;
        if printer.size.x < overhead.horizontal() {
            return None;
        }
        let mut offset = overhead.left
            + self.align
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
            button.button.draw(&printer
                .offset(position)
                .cropped(size)
                .focused(self.focus == DialogFocus::Button(i)));
            // Keep 1 blank between two buttons
            offset += size.x + 1;
            // Also keep 1 blank above the buttons
            buttons_height = max(buttons_height, size.y + 1);
        }

        Some(buttons_height)
    }

    fn draw_content(&self, printer: &Printer, buttons_height: usize) {
        // What do we have left?
        let taken = Vec2::new(0, buttons_height) + self.borders.combined()
            + self.padding.combined();

        let inner_size = match printer.size.checked_sub(taken) {
            Some(s) => s,
            None => return,
        };

        self.content.draw(&printer
            .offset(self.borders.top_left() + self.padding.top_left())
            .cropped(inner_size)
            .focused(self.focus == DialogFocus::Content));
    }

    fn draw_title(&self, printer: &Printer) {
        if !self.title.is_empty() {
            let len = self.title.width();
            if len + 4 > printer.size.x {
                return;
            }
            let spacing = 3; //minimum distance to borders
            let x = spacing
                + self.title_position
                    .get_offset(len, printer.size.x - 2 * spacing);
            printer.with_high_border(false, |printer| {
                printer.print((x - 2, 0), "┤ ");
                printer.print((x + len, 0), " ├");
            });

            printer.with_color(ColorStyle::title_primary(), |p| {
                p.print((x, 0), &self.title)
            });
        }
    }

    fn check_focus_grab(&mut self, event: &Event) {
        if let Event::Mouse {
            offset,
            position,
            event,
        } = *event
        {
            if !event.grabs_focus() {
                return;
            }

            let position = match position.checked_sub(offset) {
                None => return,
                Some(pos) => pos,
            };

            // eprintln!("Rel pos: {:?}", position);

            // Now that we have a relative position, checks for buttons?
            if let Some(i) = self.buttons.iter().position(|btn| {
                // If position fits there...
                position.fits_in_rect(btn.offset.get(), btn.button.size)
            }) {
                self.focus = DialogFocus::Button(i);
            } else if position.fits_in_rect(
                (self.padding + self.borders).top_left(),
                self.content.size,
            )
                && self.content.take_focus(Direction::none())
            {
                // Or did we click the content?
                self.focus = DialogFocus::Content;
            }
        }
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
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // First: some mouse events can instantly change the focus.
        self.check_focus_grab(&event);

        match self.focus {
            // If we are on the content, we can only go down.
            // TODO: Careful if/when we add buttons elsewhere on the dialog!
            DialogFocus::Content => self.on_event_content(event),
            // If we are on a button, we have more choice
            DialogFocus::Button(i) => self.on_event_button(event, i),
        }
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        // Dialogs aren't meant to be used in layouts, so...
        // Let's be super lazy and not even care about the focus source.
        if self.content.take_focus(source) {
            self.focus = DialogFocus::Content;
            true
        } else if !self.buttons.is_empty() {
            self.focus = DialogFocus::Button(0);
            true
        } else {
            false
        }
    }

    fn call_on_any<'a>(&mut self, selector: &Selector, callback: AnyCb<'a>) {
        self.content.call_on_any(selector, callback);
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<(), ()> {
        self.content.focus_view(selector)
    }

    fn important_area(&self, _: Vec2) -> Rect {
        self.content.important_area(self.content.size)
            + self.borders.top_left() + self.padding.top_left()
    }
}
