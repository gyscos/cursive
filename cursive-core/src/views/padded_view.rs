use crate::event::{Event, EventResult};
use crate::rect::Rect;
use crate::view::{Margins, View, ViewWrapper};
use crate::Printer;
use crate::Vec2;

/// Adds padding to another view.
///
/// This view wraps another view and adds some padding.
///
/// The wrapped view will see a reduced space available.
///
/// # Examples
///
/// ```rust
/// # use cursive_core::views::{TextView, PaddedView};
/// // Adds 2 columns of padding to the left and to the right.
///
/// // lrtb = Left, Right, Top, Bottom
/// let view = PaddedView::lrtb(2, 2, 0, 0, TextView::new("Padded text"));
/// ```
pub struct PaddedView<V> {
    view: V,
    margins: Margins,
}

impl<V> PaddedView<V> {
    /// Wraps `view` in a new `PaddedView` with the given margins.
    pub fn new(margins: Margins, view: V) -> Self {
        PaddedView { view, margins }
    }

    /// Wraps `view` in a new `PaddedView` with the given margins.
    pub fn lrtb(left: usize, right: usize, top: usize, bottom: usize, view: V) -> Self {
        Self::new(Margins::lrtb(left, right, top, bottom), view)
    }

    /// Sets the margins for this view.
    pub fn set_margins(&mut self, margins: Margins) {
        // TODO: invalidate? wrap_needs_relayout?
        self.margins = margins;
    }

    inner_getters!(self.view: V);
}

impl<V: View> ViewWrapper for PaddedView<V> {
    wrap_impl!(self.view: V);

    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        let margins = self.margins.combined();
        self.view.required_size(req.saturating_sub(margins)) + margins
    }

    fn wrap_layout(&mut self, size: Vec2) {
        let margins = self.margins.combined();
        self.view.layout(size.saturating_sub(margins));
    }

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        let padding = self.margins.top_left();
        self.view.on_event(event.relativized(padding))
    }

    fn wrap_draw(&self, printer: &Printer) {
        let top_left = self.margins.top_left();
        let bot_right = self.margins.bot_right();
        let printer = &printer.offset(top_left).shrinked(bot_right);
        self.view.draw(printer);
    }

    fn wrap_important_area(&self, view_size: Vec2) -> Rect {
        let inner_size = view_size.saturating_sub(self.margins.combined());
        self.view.important_area(inner_size) + self.margins.top_left()
    }
}

#[crate::blueprint(PaddedView::new(margins, view))]
struct Blueprint {
    margins: Margins,
    view: crate::views::BoxedView,
}

crate::manual_blueprint!(with padding, |config, context| {
    let margins = context.resolve(config)?;
    Ok(move |view| PaddedView::new(margins, view))
});
