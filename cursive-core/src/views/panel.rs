use crate::Printer;
use crate::Vec2;
use crate::With;
use crate::align::*;
use crate::event::{Event, EventResult};
use crate::rect::Rect;
use crate::style::PaletteStyle;
use crate::utils::markup::StyledString;
use crate::view::{View, ViewWrapper};

/// Draws a border around a wrapped view.
#[derive(Debug)]
pub struct Panel<V> {
    // Inner view
    view: V,

    // Possibly empty title.
    title: StyledString,

    // Where to put the title position
    title_position: HAlign,

    // Whether to draw `┤` and `├` around the title
    title_delimiters: bool,

    // `true` when we needs to relayout
    invalidated: bool,
}

new_default!(Panel<V: Default>);

/// Minimum distance between title and borders.
const TITLE_SPACING: usize = 3;

impl<V> Panel<V> {
    /// Creates a new panel around the given view.
    pub fn new(view: V) -> Self {
        Panel {
            view,
            title: StyledString::new(),
            title_position: HAlign::Center,
            title_delimiters: true,
            invalidated: true,
        }
    }

    /// Sets the title of the dialog.
    ///
    /// If not empty, it will be visible at the top.
    #[must_use]
    pub fn title<S: Into<StyledString>>(self, label: S) -> Self {
        self.with(|s| s.set_title(label))
    }

    /// Sets the title of the dialog.
    pub fn set_title<S: Into<StyledString>>(&mut self, label: S) {
        self.title = label.into();
        self.invalidate();
    }

    /// Sets the horizontal position of the title in the dialog.
    /// The default position is `HAlign::Center`
    #[must_use]
    pub fn title_position(self, align: HAlign) -> Self {
        self.with(|s| s.set_title_position(align))
    }

    /// Sets the horizontal position of the title in the dialog.
    /// The default position is `HAlign::Center`
    pub fn set_title_position(&mut self, align: HAlign) {
        self.title_position = align;
    }

    /// Sets whether the title is surrounded by delimiters (`┤ Title ├`).
    ///
    /// Without them, the title is only separated from the border by spaces.
    ///
    /// The default is `true`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::views::{Panel, TextView};
    ///
    /// let panel = Panel::new(TextView::new("Content"))
    ///     .title("Title")
    ///     .title_delimiters(false);
    /// ```
    #[must_use]
    pub fn title_delimiters(self, delimiters: bool) -> Self {
        self.with(|s| s.set_title_delimiters(delimiters))
    }

    /// Sets whether the title is surrounded by delimiters (`┤ Title ├`).
    ///
    /// The default is `true`.
    pub fn set_title_delimiters(&mut self, delimiters: bool) {
        self.title_delimiters = delimiters;
    }

    fn draw_title(&self, printer: &Printer) {
        if !self.title.is_empty() {
            let available = match printer.size.x.checked_sub(2 * TITLE_SPACING) {
                Some(available) => available,
                None => return, /* Panel is too small to even write the decoration. */
            };
            let len = std::cmp::min(self.title.width(), available);
            let x = TITLE_SPACING + self.title_position.get_offset(len, available);

            printer
                .offset((x, 0))
                .cropped((len, 1))
                .with_style(PaletteStyle::TitlePrimary, |p| {
                    p.print_styled((0, 0), &self.title)
                });
            let (left, right) = if self.title_delimiters {
                ("┤ ", " ├")
            } else {
                // Only leave a gap in the border.
                ("─ ", " ─")
            };
            printer.with_high_border(false, |printer| {
                printer.print((x - 2, 0), left);
                printer.print((x + len, 0), right);
            });
        }
    }

    fn invalidate(&mut self) {
        self.invalidated = true;
    }

    inner_getters!(self.view: V);
}

impl<V: View> ViewWrapper for Panel<V> {
    wrap_impl!(self.view: V);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        self.view.on_event(event.relativized((1, 1)))
    }

    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        // TODO: make borders conditional?
        let req = req.saturating_sub((2, 2));

        let size = self.view.required_size(req) + (2, 2);
        if self.title.is_empty() {
            size
        } else {
            let title_width = self.title.width() + 2 * TITLE_SPACING;
            size.or_max((title_width, 0))
        }
    }

    fn wrap_layout_key(&self) -> u64 {
        crate::view::combine_layout_key(
            crate::view::layout_key_seed::<Self>(),
            &(self.view.layout_key(), self.title.width()),
        )
    }

    fn wrap_draw(&self, printer: &Printer) {
        printer.print_box((0, 0), printer.size, true);
        self.draw_title(printer);

        let printer = printer.offset((1, 1)).shrinked((1, 1));
        self.view.draw(&printer);
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.invalidated = false;
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

#[crate::blueprint(Panel::new(view))]
struct Blueprint {
    view: crate::views::BoxedView,

    title: Option<StyledString>,
    title_position: Option<HAlign>,
    title_delimiters: Option<bool>,
}

// TODO: reduce code duplication between blueprints for the same view.
crate::manual_blueprint!(with panel, |config, context| {
    let title = match config {
        crate::builder::Config::String(_) => context.resolve(config)?,
        crate::builder::Config::Object(config) => {
            match config.get("title") {
                Some(title) => context.resolve(title)?,
                None => StyledString::new()
            }
        }
        _ => StyledString::new(),
    };

    let title_position = context.resolve(&config["title_position"])?;
    let title_delimiters: Option<bool> = context.resolve(&config["title_delimiters"])?;

    Ok(move |view| {
        let mut panel = crate::views::Panel::new(view).title(title);

        if let Some(title_position) = title_position {
            panel.set_title_position(title_position);
        }

        if let Some(title_delimiters) = title_delimiters {
            panel.set_title_delimiters(title_delimiters);
        }

        panel
    })
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::theme::Theme;
    use crate::views::DummyView;
    use parking_lot::RwLock;

    fn top_row(panel: &Panel<DummyView>) -> String {
        let theme = Theme::retro();
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(14, 3));
        let buffer = RwLock::new(buffer);
        panel.draw(&Printer::new((14, 3), &theme, &buffer));

        let buffer = buffer.read();
        (0..14)
            .map(|x| buffer.cell_text(Vec2::new(x, 0)).unwrap_or(" ").to_string())
            .collect()
    }

    #[test]
    fn title_delimiters() {
        let panel = Panel::new(DummyView)
            .title("Hi")
            .title_position(HAlign::Left);
        assert_eq!(top_row(&panel), "┌┤ Hi ├──────┐");

        let panel = panel.title_delimiters(false);
        assert_eq!(top_row(&panel), "┌─ Hi ───────┐");
    }
}
