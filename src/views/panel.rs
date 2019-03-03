use crate::align::*;
use crate::event::{Event, EventResult};
use crate::rect::Rect;
use crate::theme::ColorStyle;
use crate::vec::Vec2;
use crate::view::{View, ViewWrapper};
use crate::Printer;
use crate::With;
use unicode_width::UnicodeWidthStr;

/// Draws a border around a wrapped view.
#[derive(Debug)]
pub struct Panel<V: View> {
    // Inner view
    view: V,

    // Possibly empty title.
    title: String,

    // Where to put the title position
    title_position: HAlign,

    // `true` when we needs to relayout
    invalidated: bool,
}

impl<V: View> Panel<V> {
    /// Creates a new panel around the given view.
    pub fn new(view: V) -> Self {
        Panel {
            view,
            title: String::new(),
            title_position: HAlign::Center,
            invalidated: true,
        }
    }

    inner_getters!(self.view: V);

    /// Sets the title of the dialog.
    ///
    /// If not empty, it will be visible at the top.
    pub fn title<S: Into<String>>(self, label: S) -> Self {
        self.with(|s| s.set_title(label))
    }

    /// Sets the title of the dialog.
    pub fn set_title<S: Into<String>>(&mut self, label: S) {
        self.title = label.into();
        self.invalidate();
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

    fn draw_title(&self, printer: &Printer<'_, '_>) {
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

    fn invalidate(&mut self) {
        self.invalidated = true;
    }
}

impl<V: View> ViewWrapper for Panel<V> {
    wrap_impl!(self.view: V);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        self.view.on_event(event.relativized((1, 1)))
    }

    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        // TODO: make borders conditional?
        let req = req.saturating_sub((2, 2));

        self.view.required_size(req) + (2, 2)
    }

    fn wrap_draw(&self, printer: &Printer<'_, '_>) {
        printer.print_box((0, 0), printer.size, true);
        self.draw_title(&printer);

        let printer = printer.offset((1, 1)).shrinked((1, 1));
        self.view.draw(&printer);
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.view.layout(size.saturating_sub((2, 2)));
    }

    fn wrap_important_area(&self, size: Vec2) -> Rect {
        let inner_size = size.saturating_sub((2, 2));
        self.view.important_area(inner_size) + (1, 1)
    }

    fn wrap_needs_relayout(&self) -> bool {
        self.invalidated || self.view.needs_relayout()
    }
}
