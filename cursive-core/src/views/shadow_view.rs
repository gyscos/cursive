use crate::event::{Event, EventResult};
use crate::rect::Rect;
use crate::style::PaletteStyle;
use crate::view::{View, ViewWrapper};
use crate::Printer;
use crate::Vec2;

/// Wrapper view that adds a shadow.
///
/// It reserves a 1 pixel border on each side.
pub struct ShadowView<T> {
    view: T,
    top_padding: bool,
    left_padding: bool,
    // TODO: invalidate if we change the padding? wrap_needs_relayout?
}

new_default!(ShadowView<T: Default>);

impl<T> ShadowView<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        ShadowView {
            view,
            top_padding: true,
            left_padding: true,
        }
    }

    /// Return the total padding for this view (include both sides)
    fn padding(&self) -> Vec2 {
        // We always need (1, 1) for the shadow.
        self.top_left_padding() + (1, 1)
    }

    fn top_left_padding(&self) -> Vec2 {
        Vec2::new(self.left_padding as usize, self.top_padding as usize)
    }

    /// If set, adds an empty column to the left of the view.
    ///
    /// Default to true.
    #[must_use]
    pub fn left_padding(mut self, value: bool) -> Self {
        self.left_padding = value;
        self
    }

    /// If set, adds an empty row at the top of the view.
    ///
    /// Default to true.
    #[must_use]
    pub fn top_padding(mut self, value: bool) -> Self {
        self.top_padding = value;
        self
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for ShadowView<T> {
    wrap_impl!(self.view: T);

    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        // Make sure req >= offset
        let offset = self.padding();
        self.view.required_size(req.saturating_sub(offset)) + offset
    }

    fn wrap_layout(&mut self, size: Vec2) {
        let offset = self.padding();
        self.view.layout(size.saturating_sub(offset));
    }

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        let padding = self.top_left_padding();
        self.view.on_event(event.relativized(padding))
    }

    fn wrap_draw(&self, printer: &Printer) {
        if printer.size.y <= self.top_padding as usize
            || printer.size.x <= self.left_padding as usize
        {
            // Nothing to do if there's no place to draw.
            return;
        }

        // Skip the first row/column
        let offset = Vec2::new(self.left_padding as usize, self.top_padding as usize);
        let printer = &printer.offset(offset);
        if printer.theme.shadow {
            let h = printer.size.y;
            let w = printer.size.x;

            if h == 0 || w == 0 {
                return;
            }

            printer.with_style(PaletteStyle::Shadow, |printer| {
                printer.print_hline((1, h - 1), w - 1, " ");
                printer.print_vline((w - 1, 1), h - 1, " ");
            });
        }

        // Draw the view background
        let printer = printer.shrinked((1, 1));
        self.view.draw(&printer);
    }

    fn wrap_important_area(&self, view_size: Vec2) -> Rect {
        self.view
            .important_area(view_size.saturating_sub(self.padding()))
            + self.top_left_padding()
    }
}

#[crate::blueprint(ShadowView::new(view))]
struct Blueprint {
    view: crate::views::BoxedView,
}

crate::manual_blueprint!(with shadow, |_, _| Ok(ShadowView::new));
