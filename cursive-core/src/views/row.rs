use crate::Printer;
use crate::Vec2;
use crate::align::HAlign;
use crate::event::{AnyCb, Event, EventResult};
use crate::rect::Rect;
use crate::utils::markup::StyledString;
use crate::view::{
    CannotFocus, IntoBoxedView, Selector, View, ViewNotFound, combine_layout_key, fresh_layout_key,
    layout_key_seed,
};
use crate::views::{BoxedView, DummyView, TextView};

/// A label and a value side by side.
///
/// The label takes its natural width (or a fixed one), and the value takes
/// the rest of the row.
///
/// The label is usually text ([`Row::new`], [`Row::text`]), but it can be any
/// view ([`Row::from_views`]): a [`TextView::live`] gives a label that
/// follows live state, for example one whose color changes with a value.
///
/// The value decides how it uses its cell. A [`TextView`] aligned to the
/// right pushes the value to the right edge, so that stacked rows of the same
/// width line their values up without knowing about each other. This is what
/// [`Row::text`] builds.
///
/// To line up the *start* of the values in a form instead, give every row the
/// same [`label_width`](Row::label_width), or use a
/// [`ListView`](crate::views::ListView), which knows all its labels.
///
/// # Interaction
///
/// The label is never interactive: it does not take focus and receives no
/// event, even if it is a focusable view like a [`Button`](crate::views::Button).
/// Focus, events (with their position relative to the value), and the
/// important area all go to the value view.
///
/// [`call_on_any`](View::call_on_any) reaches both views, so a named view
/// inside the label can be found. [`focus_view`](View::focus_view) only
/// looks in the value, since the label cannot be focused.
///
/// # Sizes
///
/// The label column is as wide as the label requires (asked with the
/// constraint of the row, so a wrapping label only uses several lines when
/// the row is narrow), or [`label_width`](Row::label_width) if it is set. The
/// value gets what is left of the width, after the column and the
/// [gap](Row::gap).
///
/// The row keeps the column it used for each size it returned from
/// `required_size`, and uses it again when it is laid out with that size: a
/// parent that gives the row exactly what it asked for always gets the value
/// it was promised, even when a wrapping label would take a different width
/// if it were measured again with the size of the row. For any other size,
/// the label is measured with the size it is laid out with.
///
/// The row is as tall as the taller of the two, and at least 1. Both views
/// are laid out with the full height of the row. A shorter view is drawn at
/// the *top* of the row, unless it aligns itself (a [`TextView`] with a
/// [`v_align`](TextView::v_align), for example).
///
/// A label wider than its column (a fixed width, or a row narrower than the
/// label) is cropped: it never spills into the value's cells. When the row is
/// too narrow for the label and the gap, the value has a zero-width cell: it
/// is laid out with a width of 0 and is not drawn, but it is still the
/// focused view and still receives events.
///
/// # Alignment of the label
///
/// With a fixed [`label_width`](Row::label_width) wider than the label,
/// [`label_align`](Row::label_align) places a *text* label (from
/// [`Row::new`], [`Row::text`] or [`Row::set_label`]) inside its column. A
/// view label is given the whole column and aligns itself: `label_align` has
/// no effect on it.
///
/// # Examples
///
/// ```rust
/// use cursive_core::align::HAlign;
/// use cursive_core::views::{LinearLayout, Row, TextView};
///
/// // The values are pushed to the right edge.
/// let view = LinearLayout::vertical()
///     .child(Row::text("Latency", "22ms"))
///     .child(Row::text("Throughput", "1.5GB/s"));
///
/// // A text label, and a live value refreshed on every layout.
/// let row = Row::new("Uptime", TextView::live(|| "3 days").h_align(HAlign::Right));
///
/// // A live label too: it can change its text, or its color, on its own.
/// let row = Row::from_views(
///     TextView::live(|| "● eu-west"),
///     TextView::live(|| "22ms").h_align(HAlign::Right),
/// );
///
/// // A fixed label column, with the label aligned to the right of it.
/// let row = Row::text("Host", "example.org")
///     .label_width(12)
///     .label_align(HAlign::Right)
///     .gap(2);
/// ```
pub struct Row {
    label: BoxedView,
    // Whether `label` is a `TextView` made from text by this row, which the
    // row aligns itself.
    text_label: bool,
    label_width: Option<usize>,
    label_align: HAlign,
    gap: usize,
    value: BoxedView,

    // Width of the label column, as of the last layout.
    column: usize,
    // Changes every time the label is replaced.
    label_epoch: u64,
    // Set when the row itself changed since the last layout.
    changed: bool,
    // The label columns that `required_size` used, by the size it returned.
    // Only valid for the `layout_key` it was recorded with.
    measured: Option<(u64, Vec<(Vec2, usize)>)>,
}

/// How many answers of `required_size` a row remembers.
const MEASURED: usize = 8;

/// A non-wrapping text view.
fn text_label(label: StyledString) -> BoxedView {
    BoxedView::new(Box::new(TextView::new(label).no_wrap()))
}

impl Row {
    /// Creates a new row with a text label and a value.
    ///
    /// The label is a [`TextView`] that does not wrap. By default it has its
    /// natural width, and is separated from the value by a gap of 1 cell.
    pub fn new(label: impl Into<StyledString>, value: impl IntoBoxedView) -> Self {
        let mut row = Self::from_views(DummyView, value);
        row.set_label(label);
        row
    }

    /// Creates a new row with a text label and a text value pushed to the right.
    ///
    /// The value is a right-aligned [`TextView`] that does not wrap.
    pub fn text(label: impl Into<StyledString>, value: impl Into<StyledString>) -> Self {
        Self::new(label, TextView::new(value).h_align(HAlign::Right).no_wrap())
    }

    /// Creates a new row with any view as the label.
    ///
    /// For example, a [`TextView::live`] for a label that follows live state.
    ///
    /// The label view is given the whole label column and aligns itself:
    /// [`label_align`](Self::label_align) has no effect on it. It is never
    /// focused and receives no event.
    pub fn from_views(label: impl IntoBoxedView, value: impl IntoBoxedView) -> Self {
        Row {
            label: BoxedView::new(label.into_boxed_view()),
            text_label: false,
            label_width: None,
            label_align: HAlign::Left,
            gap: 1,
            value: BoxedView::new(value.into_boxed_view()),
            column: 0,
            label_epoch: fresh_layout_key(),
            changed: true,
            measured: None,
        }
    }

    /// Replaces the label with a text one.
    pub fn set_label(&mut self, label: impl Into<StyledString>) {
        self.label = text_label(label.into());
        self.text_label = true;
        self.label_epoch = fresh_layout_key();
        self.changed = true;
    }

    /// Replaces the label with a text one.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn label(self, label: impl Into<StyledString>) -> Self {
        crate::With::with(self, |s| s.set_label(label))
    }

    /// Replaces the label with a view.
    ///
    /// See [`Row::from_views`].
    pub fn set_label_view(&mut self, label: impl IntoBoxedView) {
        self.label = BoxedView::new(label.into_boxed_view());
        self.text_label = false;
        self.label_epoch = fresh_layout_key();
        self.changed = true;
    }

    /// Replaces the label with a view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn label_view(self, label: impl IntoBoxedView) -> Self {
        crate::With::with(self, |s| s.set_label_view(label))
    }

    /// Returns the label view.
    ///
    /// For a text label, this is a [`TextView`].
    pub fn get_label_view(&self) -> &BoxedView {
        &self.label
    }

    /// Returns the label view, mutably.
    pub fn get_label_view_mut(&mut self) -> &mut BoxedView {
        &mut self.label
    }

    /// Returns the value view.
    pub fn get_value(&self) -> &BoxedView {
        &self.value
    }

    /// Returns the value view, mutably.
    pub fn get_value_mut(&mut self) -> &mut BoxedView {
        &mut self.value
    }

    /// Sets the number of cells between the label and the value.
    ///
    /// The default is 1.
    pub fn set_gap(&mut self, gap: usize) {
        self.gap = gap;
        self.changed = true;
    }

    /// Sets the number of cells between the label and the value.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn gap(self, gap: usize) -> Self {
        crate::With::with(self, |s| s.set_gap(gap))
    }

    /// Sets the width of the label column.
    ///
    /// With `None` (the default), the column is as wide as the label requires.
    /// Otherwise, the label is given a column of this width: a text label is
    /// aligned in it (see [`label_align`](Self::set_label_align)), a view
    /// label aligns itself. A label that is wider is cropped.
    pub fn set_label_width(&mut self, width: impl Into<Option<usize>>) {
        self.label_width = width.into();
        self.changed = true;
    }

    /// Sets the width of the label column.
    ///
    /// Accepts a width or an `Option`. Chainable variant.
    #[must_use]
    pub fn label_width(self, width: impl Into<Option<usize>>) -> Self {
        crate::With::with(self, |s| s.set_label_width(width))
    }

    /// Sets how a text label is aligned inside a fixed-width label column.
    ///
    /// The default is [`HAlign::Left`]. With a one-line label and no fixed
    /// width, the column fits the label and there is nothing to align. It
    /// also aligns the shorter lines of a multi-line label inside the column,
    /// whether the width is fixed or not. It only applies to text
    /// labels: a view label is given the whole column and aligns itself.
    pub fn set_label_align(&mut self, align: HAlign) {
        self.label_align = align;
        self.changed = true;
    }

    /// Sets how a text label is aligned inside a fixed-width label column.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn label_align(self, align: HAlign) -> Self {
        crate::With::with(self, |s| s.set_label_align(align))
    }

    /// Column where the value starts.
    fn value_x(&self) -> usize {
        self.column + self.gap
    }

    /// Measures the label: the width of the column, and the height it needs.
    fn measure_label(&mut self, req: Vec2) -> Vec2 {
        match self.label_width {
            Some(width) => {
                let size = self.label.required_size(Vec2::new(width, req.y));
                Vec2::new(width, size.y)
            }
            None => self.label.required_size(req),
        }
    }

    /// Remembers that `required_size` returned `size` with this label column.
    fn remember(&mut self, size: Vec2, column: usize) {
        let key = self.layout_key();
        let (_, measured) = match &mut self.measured {
            Some(measured) if measured.0 == key => measured,
            other => other.insert((key, Vec::new())),
        };
        measured.retain(|&(known, _)| known != size);
        if measured.len() == MEASURED {
            measured.remove(0);
        }
        measured.push((size, column));
    }

    /// Returns the label column `required_size` used to return `size`, if it still applies.
    fn recall(&self, size: Vec2) -> Option<usize> {
        let (key, measured) = self.measured.as_ref()?;
        if *key != self.layout_key() {
            return None;
        }
        measured
            .iter()
            .find(|&&(known, _)| known == size)
            .map(|&(_, column)| column)
    }

    /// Draws a text label, aligned in the label column.
    ///
    /// The label is cropped to the column and to the printer.
    fn draw_text_label(&self, printer: &Printer, label: &TextView) {
        let content = label.get_content();
        if content.source().contains('\n') {
            for (y, line) in split_lines(&content).iter().enumerate() {
                printer.print_styled_aligned((0, y), self.column, self.label_align, line);
            }
        } else {
            printer.print_styled_aligned((0, 0), self.column, self.label_align, &*content);
        }
    }
}

/// Splits a styled string on its newlines, keeping the styles.
fn split_lines(text: &StyledString) -> Vec<StyledString> {
    let mut lines = vec![StyledString::new()];
    for span in text.spans() {
        for (i, part) in span.content.split('\n').enumerate() {
            if i > 0 {
                lines.push(StyledString::new());
            }
            if !part.is_empty() {
                lines.last_mut().unwrap().append_styled(part, *span.attr);
            }
        }
    }
    lines
}

impl View for Row {
    fn draw(&self, printer: &Printer) {
        if printer.size.x == 0 || printer.size.y == 0 {
            return;
        }

        // The column is wider than the printer when the row is too narrow:
        // the printer does the cropping.
        let label_printer = printer
            .cropped((self.column, printer.size.y))
            .focused(false);
        match self.label.downcast_ref::<TextView>() {
            Some(label) if self.text_label => self.draw_text_label(&label_printer, label),
            _ => self.label.draw(&label_printer),
        }

        let x = self.value_x();
        let width = printer.size.x.saturating_sub(x);
        if width > 0 {
            self.value
                .draw(&printer.offset((x, 0)).cropped((width, printer.size.y)));
        }
    }

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        let label = self.measure_label(req);
        let x = label.x + self.gap;
        let remaining = Vec2::new(req.x.saturating_sub(x), req.y);
        let value = self.value.required_size(remaining);
        let size = Vec2::new(x + value.x, value.y.max(label.y).max(1));
        if self.label_width.is_none() {
            self.remember(size, label.x);
        }
        size
    }

    fn layout(&mut self, size: Vec2) {
        self.changed = false;
        self.column = match self.recall(size) {
            Some(column) => column,
            None => self.measure_label(size).x,
        };
        self.label.layout(Vec2::new(self.column, size.y));
        let width = size.x.saturating_sub(self.value_x());
        self.value.layout(Vec2::new(width, size.y));
    }

    fn needs_relayout(&self) -> bool {
        self.changed || self.label.needs_relayout() || self.value.needs_relayout()
    }

    fn layout_key(&self) -> u64 {
        combine_layout_key(
            layout_key_seed::<Self>(),
            &(
                self.label_epoch,
                self.label.layout_key(),
                self.label_width,
                self.gap,
                self.value.layout_key(),
            ),
        )
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        let offset = Vec2::new(self.value_x(), 0);
        self.value.on_event(event.relativized(offset))
    }

    fn take_focus(
        &mut self,
        source: crate::direction::Direction,
    ) -> Result<EventResult, CannotFocus> {
        self.value.take_focus(source)
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        self.label.call_on_any(selector, callback);
        self.value.call_on_any(selector, callback);
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.value.focus_view(selector)
    }

    fn important_area(&self, size: Vec2) -> Rect {
        let x = self.value_x();
        let inner = Vec2::new(size.x.saturating_sub(x), size.y);
        self.value.important_area(inner) + (x, 0)
    }
}

/// A label in a config: text, or a view.
#[cfg(feature = "builder")]
enum LabelConfig {
    Text(StyledString),
    View(BoxedView),
}

#[cfg(feature = "builder")]
impl crate::builder::Resolvable for LabelConfig {
    fn from_config(
        config: &crate::builder::Config,
        context: &crate::builder::Context,
    ) -> Result<Self, crate::builder::Error> {
        // A string is text (not the name of a view), an object is a view.
        match config {
            crate::builder::Config::Object(_) => context.resolve(config).map(LabelConfig::View),
            _ => context.resolve(config).map(LabelConfig::Text),
        }
    }

    fn from_any(any: Box<dyn std::any::Any>) -> Option<Self> {
        let any = match any.downcast::<BoxedView>() {
            Ok(view) => return Some(LabelConfig::View(*view)),
            Err(any) => any,
        };
        <StyledString as crate::builder::Resolvable>::from_any(any).map(LabelConfig::Text)
    }
}

#[cfg(feature = "builder")]
impl Row {
    fn from_label_config(label: LabelConfig, value: BoxedView) -> Self {
        match label {
            // Unwrap the views: a `BoxedView` would be boxed a second time otherwise.
            LabelConfig::Text(label) => Self::new(label, value.unwrap()),
            LabelConfig::View(label) => Self::from_views(label.unwrap(), value.unwrap()),
        }
    }
}

#[crate::blueprint(Row::from_label_config(label, value))]
struct Blueprint {
    label: LabelConfig,
    value: BoxedView,

    gap: Option<usize>,
    label_width: Option<usize>,
    label_align: Option<HAlign>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::align::VAlign;
    use crate::buffer::PrintBuffer;
    use crate::event::{MouseButton, MouseEvent};
    use crate::style::Color;
    use crate::theme::Theme;
    use crate::view::{Finder, Nameable};
    use crate::views::{Button, Checkbox, EditView, Gauge, LinearLayout, ProgressBar, ScrollView};
    use parking_lot::RwLock;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    const RED: Color = Color::Rgb(255, 0, 0);

    /// Lays out the view at the given size, and draws it.
    fn draw_at(view: &mut dyn View, size: Vec2) -> RwLock<PrintBuffer> {
        view.layout(size);
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        let buffer = RwLock::new(buffer);
        view.draw(&Printer::new(size, &Theme::retro(), &buffer));
        buffer
    }

    /// The lines of text, with `.` for the cells that are empty or spaces.
    fn lines(view: &mut dyn View, size: impl Into<Vec2>) -> Vec<String> {
        let size = size.into();
        let buffer = draw_at(view, size);
        let buffer = buffer.read();
        (0..size.y)
            .map(|y| {
                (0..size.x)
                    .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                        None | Some("") | Some(" ") => ".",
                        Some(text) => text,
                    })
                    .collect()
            })
            .collect()
    }

    /// Draws at the natural width and returns the first line.
    fn natural(row: &mut Row) -> String {
        let size = row.required_size(Vec2::new(100, 10));
        lines(row, size).remove(0)
    }

    /// The text of a text label.
    fn label_source(row: &Row) -> String {
        row.get_label_view()
            .downcast_ref::<TextView>()
            .unwrap()
            .get_content()
            .source()
            .to_string()
    }

    fn edit_cursor(row: &Row) -> usize {
        row.get_value()
            .downcast_ref::<EditView>()
            .unwrap()
            .get_cursor()
    }

    #[test]
    fn natural_label_width() {
        let mut row = Row::new("ab", TextView::new("cd"));
        assert_eq!(row.required_size(Vec2::new(50, 5)), Vec2::new(5, 1));
        assert_eq!(natural(&mut row), "ab.cd");
        // The value is the one to take the extra room.
        assert_eq!(lines(&mut row, (8, 1)), ["ab.cd..."]);
    }

    #[test]
    fn fixed_label_width_and_alignment() {
        let row = || Row::new("ab", TextView::new("cd")).label_width(6);
        assert_eq!(lines(&mut row(), (9, 1)), ["ab.....cd"]);
        assert_eq!(
            lines(&mut row().label_align(HAlign::Right), (9, 1)),
            ["....ab.cd"]
        );
        assert_eq!(
            lines(&mut row().label_align(HAlign::Center), (9, 1)),
            ["..ab...cd"]
        );
        assert_eq!(row().required_size(Vec2::new(50, 5)), Vec2::new(9, 1));

        let mut reset = row();
        reset.set_label_width(None);
        assert_eq!(reset.required_size(Vec2::new(50, 5)), Vec2::new(5, 1));
    }

    #[test]
    fn gap() {
        let mut row = Row::new("ab", TextView::new("cd")).gap(3);
        assert_eq!(natural(&mut row), "ab...cd");
        row.set_gap(0);
        assert_eq!(natural(&mut row), "abcd");
    }

    #[test]
    fn styled_label_keeps_its_styles() {
        let mut label = StyledString::styled("ab", RED);
        label.append("cd");
        let mut row = Row::new(label, TextView::new("v")).label_width(5);
        let buffer = draw_at(&mut row, Vec2::new(7, 1));
        let buffer = buffer.read();
        let front = |x| buffer.cell_style(Vec2::new(x, 0)).unwrap().color.front;
        assert_eq!(front(0), RED);
        assert_eq!(front(1), RED);
        assert_ne!(front(2), RED);
        assert_eq!(buffer.cell_text(Vec2::new(3, 0)), Some("d"));
        assert_eq!(buffer.cell_text(Vec2::new(6, 0)), Some("v"));
    }

    #[test]
    fn text_aligns_to_the_right() {
        let mut row = Row::text("Lat", "22ms");
        assert_eq!(natural(&mut row), "Lat.22ms");
        assert_eq!(lines(&mut row, (12, 1)), ["Lat.....22ms"]);
        // Does not wrap, even when squeezed.
        assert_eq!(row.required_size(Vec2::new(6, 5)).y, 1);
    }

    #[test]
    fn stacked_rows_line_up() {
        let mut layout = LinearLayout::vertical()
            .child(Row::text("Latency", "22ms"))
            .child(Row::text("Throughput", "1.5GB/s"))
            .child(Row::text("Errors", "0"));
        let size = layout.required_size(Vec2::new(40, 10));
        assert_eq!(size, Vec2::new(18, 3));
        assert_eq!(
            lines(&mut layout, size),
            [
                "Latency.......22ms",
                "Throughput.1.5GB/s",
                "Errors...........0"
            ]
        );
        // Wider: the values stay at the right edge.
        assert_eq!(
            lines(&mut layout, (20, 3)),
            [
                "Latency.........22ms",
                "Throughput...1.5GB/s",
                "Errors.............0"
            ]
        );
    }

    #[test]
    fn cropped_when_too_narrow() {
        let mut row = Row::new("abcdef", TextView::new("xy")).label_width(3);
        // Fixed column narrower than the label.
        assert_eq!(lines(&mut row, (8, 1)), ["abc.xy.."]);
        // Right-aligned labels keep their end.
        row.set_label_align(HAlign::Right);
        assert_eq!(lines(&mut row, (8, 1)), ["def.xy.."]);

        // A row narrower than the natural label: the value gets no cell.
        let mut row = Row::new("abcdef", TextView::new("xy"));
        assert_eq!(lines(&mut row, (4, 1)), ["abcd"]);
        assert_eq!(lines(&mut row, (6, 1)), ["abcdef"]);
        assert_eq!(lines(&mut row, (7, 1)), ["abcdef."]);
        assert_eq!(lines(&mut row, (8, 1)), ["abcdef.x"]);
        assert_eq!(lines(&mut row, (0, 1)), [""]);
    }

    #[test]
    fn wide_characters_in_the_label() {
        // Each of these takes 2 cells.
        let mut row = Row::new("日本", TextView::new("v"));
        assert_eq!(row.required_size(Vec2::new(50, 1)), Vec2::new(6, 1));
        assert_eq!(natural(&mut row), "日.本..v");

        // A wide character is never split.
        let mut row = Row::new("日本", TextView::new("v")).label_width(3);
        assert_eq!(lines(&mut row, (6, 1)), ["日...v."]);
    }

    #[test]
    fn focusable_value_gets_focus_and_events() {
        let mut row = Row::new("name", EditView::new().content("hello")).label_width(6);
        assert!(row.take_focus(crate::direction::Direction::none()).is_ok());
        row.layout(Vec2::new(20, 1));
        assert_eq!(edit_cursor(&row), 5);

        assert!(row.on_event(Event::Char('!')).is_consumed());
        let edit = row.get_value().downcast_ref::<EditView>().unwrap();
        assert_eq!(*edit.get_content(), "hello!");

        // A click on the third cell of the value: the cursor goes there.
        let value_x = 6 + 1;
        row.on_event(Event::Mouse {
            offset: Vec2::new(0, 0),
            position: Vec2::new(value_x + 2, 0),
            event: MouseEvent::Press(MouseButton::Left),
        });
        assert_eq!(edit_cursor(&row), 2);

        // With an offset from the parent.
        row.on_event(Event::Mouse {
            offset: Vec2::new(3, 4),
            position: Vec2::new(3 + value_x + 1, 4),
            event: MouseEvent::Press(MouseButton::Left),
        });
        assert_eq!(edit_cursor(&row), 1);
    }

    #[test]
    fn button_receives_clicks_at_the_right_offset() {
        let clicked = Arc::new(AtomicBool::new(false));
        let flag = clicked.clone();
        let mut row = Row::new(
            "label",
            Button::new("Ok", move |_| flag.store(true, Ordering::Relaxed)),
        );
        let size = row.required_size(Vec2::new(40, 5));
        row.layout(size);
        let click = |x| Event::Mouse {
            offset: Vec2::zero(),
            position: Vec2::new(x, 0),
            event: MouseEvent::Release(MouseButton::Left),
        };
        // On the label: nothing.
        row.on_event(click(1)).process(&mut crate::Cursive::new());
        assert!(!clicked.load(Ordering::Relaxed));
        // On the button.
        let res = row.on_event(click(6 + 1));
        assert!(res.is_consumed());
        res.process(&mut crate::Cursive::new());
        assert!(clicked.load(Ordering::Relaxed));
    }

    #[test]
    fn important_area_is_the_values() {
        let mut row = Row::new("name", EditView::new()).label_width(6);
        row.layout(Vec2::new(20, 1));
        let area = row.important_area(Vec2::new(20, 1));
        assert!(area.top_left().x >= 7);
    }

    #[test]
    fn multi_line_value() {
        let mut row = Row::new("ab", TextView::new("1\n22\n333"));
        assert_eq!(row.required_size(Vec2::new(50, 10)), Vec2::new(6, 3));
        // The label is only drawn on the first line.
        assert_eq!(lines(&mut row, (6, 3)), ["ab.1..", "...22.", "...333"]);
    }

    #[test]
    fn layout_key_changes() {
        let mut row = Row::new("ab", TextView::new("cd"));
        let mut previous = vec![row.layout_key()];
        row.set_label("abc");
        previous.push(row.layout_key());
        row.set_gap(2);
        previous.push(row.layout_key());
        row.set_label_width(Some(8));
        previous.push(row.layout_key());
        for (i, key) in previous.iter().enumerate() {
            assert!(!previous[i + 1..].contains(key), "key {i} did not change");
        }

        // Settings that only change the drawing keep the key.
        let before = row.layout_key();
        row.set_label_align(HAlign::Right);
        assert_eq!(row.layout_key(), before);

        // The value counts too.
        row.get_value_mut()
            .downcast_mut::<TextView>()
            .unwrap()
            .set_content("longer");
        assert_ne!(row.layout_key(), before);
    }

    #[test]
    fn relayout_after_changes() {
        let mut row = Row::new("ab", TextView::new("cd"));
        assert!(row.needs_relayout());
        row.layout(Vec2::new(10, 1));
        assert!(!row.changed);
        row.set_label("abc");
        assert!(row.needs_relayout());
        row.layout(Vec2::new(10, 1));
        row.set_gap(3);
        assert!(row.needs_relayout());
    }

    #[test]
    fn in_a_vertical_linear_layout() {
        let mut layout = LinearLayout::vertical()
            .child(Row::new("a", TextView::new("1")).label_width(4))
            .child(Row::new("bbbb", TextView::new("2\n3")));
        let size = layout.required_size(Vec2::new(20, 20));
        assert_eq!(size, Vec2::new(6, 3));
        assert_eq!(lines(&mut layout, size), ["a....1", "bbbb.2", ".....3"]);
    }

    #[test]
    fn accessors() {
        let mut row = Row::new("ab", TextView::new("cd")).label("xy");
        assert_eq!(label_source(&row), "xy");
        assert!(row.get_value().downcast_ref::<TextView>().is_some());
        row.get_value_mut()
            .downcast_mut::<TextView>()
            .unwrap()
            .set_content("zz");
        assert_eq!(natural(&mut row), "xy.zz");
    }

    /// A view of a fixed size that counts how many times it is measured.
    struct Probe {
        size: Vec2,
        calls: Arc<AtomicUsize>,
    }

    impl View for Probe {
        fn draw(&self, _: &Printer) {}

        fn required_size(&mut self, _: Vec2) -> Vec2 {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.size
        }
    }

    /// Measures the view, lays it out at that size, and returns the size and the lines.
    fn frame(view: &mut dyn View) -> (Vec2, Vec<String>) {
        let size = view.required_size(Vec2::new(40, 10));
        (size, lines(view, size))
    }

    #[test]
    fn view_label_natural_width() {
        let mut row = Row::from_views(TextView::new("ab"), TextView::new("cd"));
        assert_eq!(row.required_size(Vec2::new(50, 5)), Vec2::new(5, 1));
        assert_eq!(natural(&mut row), "ab.cd");
        assert_eq!(lines(&mut row, (8, 1)), ["ab.cd..."]);
        // Same thing for a label set afterwards.
        let mut row = Row::new("x", TextView::new("cd")).label_view(TextView::new("abc"));
        assert_eq!(natural(&mut row), "abc.cd");
    }

    #[test]
    fn live_label_follows_its_source() {
        let state = Arc::new(parking_lot::Mutex::new(StyledString::styled("ab", RED)));
        let label = {
            let state = state.clone();
            TextView::live(move || state.lock().clone())
        };
        let mut layout = LinearLayout::vertical()
            .child(Row::from_views(
                label,
                TextView::new("1").h_align(HAlign::Right),
            ))
            .child(Row::text("Latency", "22ms"));

        let (size, text) = frame(&mut layout);
        assert_eq!(size, Vec2::new(12, 2));
        assert_eq!(text, ["ab.........1", "Latency.22ms"]);

        // Wider, on two lines, and in another color.
        let blue = Color::Rgb(0, 0, 255);
        let key = layout.layout_key();
        *state.lock() = StyledString::styled("abcdefghijklmnop\nx", blue);
        assert!(layout.needs_relayout());
        let size = layout.required_size(Vec2::new(40, 10));
        assert_eq!(size, Vec2::new(18, 3));
        let buffer = draw_at(&mut layout, size);
        let buffer = buffer.read();
        let text = |y| -> String {
            (0..size.x)
                .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                    None | Some("") | Some(" ") => ".",
                    Some(text) => text,
                })
                .collect()
        };
        // The value moved, and the sibling is pushed down and stretched.
        assert_eq!(text(0), "abcdefghijklmnop.1");
        assert_eq!(text(1), "x.................");
        assert_eq!(text(2), "Latency.......22ms");
        let front = |x, y| buffer.cell_style(Vec2::new(x, y)).unwrap().color.front;
        assert_eq!(front(0, 0), blue);
        assert_eq!(front(0, 1), blue);
        drop(buffer);
        assert_ne!(layout.layout_key(), key);
    }

    #[test]
    fn multi_line_label_with_a_one_line_value() {
        let mut row = Row::from_views(TextView::new("a\nbc"), TextView::new("v"));
        assert_eq!(row.required_size(Vec2::new(50, 10)), Vec2::new(4, 2));
        // The value is on the first line.
        assert_eq!(lines(&mut row, (4, 2)), ["a..v", "bc.."]);

        // Unless it aligns itself, as it has the full height.
        let mut row = Row::from_views(
            TextView::new("a\nbc"),
            TextView::new("v").v_align(VAlign::Bottom),
        );
        assert_eq!(lines(&mut row, (4, 2)), ["a...", "bc.v"]);

        // A multi-line text label.
        let mut row = Row::new("a\nbc", TextView::new("v"));
        assert_eq!(row.required_size(Vec2::new(50, 10)), Vec2::new(4, 2));
        assert_eq!(lines(&mut row, (4, 2)), ["a..v", "bc.."]);
    }

    #[test]
    fn one_line_label_with_a_multi_line_value() {
        let mut row = Row::from_views(TextView::new("a"), TextView::new("1\n2\n3"));
        assert_eq!(row.required_size(Vec2::new(50, 10)), Vec2::new(3, 3));
        // The label is on the first line.
        assert_eq!(lines(&mut row, (3, 3)), ["a.1", "..2", "..3"]);

        // Unless it aligns itself.
        let mut row = Row::from_views(
            TextView::new("a").v_align(VAlign::Center),
            TextView::new("1\n2\n3"),
        );
        assert_eq!(lines(&mut row, (3, 3)), ["..1", "a.2", "..3"]);

        // The row can also be taller than both.
        let mut row = Row::from_views(TextView::new("a"), TextView::new("1"));
        assert_eq!(lines(&mut row, (3, 2)), ["a.1", "..."]);
    }

    #[test]
    fn multi_line_text_label_alignment() {
        let mut row = Row::new("ab\ncde", TextView::new("v"))
            .label_width(5)
            .label_align(HAlign::Right);
        assert_eq!(row.required_size(Vec2::new(50, 10)), Vec2::new(7, 2));
        assert_eq!(lines(&mut row, (7, 2)), ["...ab.v", "..cde.."]);
    }

    #[test]
    fn wrapping_label_uses_lines_only_when_narrow() {
        let mut row = Row::from_views(TextView::new("abc def"), TextView::new("x"));
        assert_eq!(row.required_size(Vec2::new(40, 5)), Vec2::new(9, 1));
        // A wrapping text takes all the width it is given: here the label
        // takes the whole row, and the value has no room left.
        assert_eq!(row.required_size(Vec2::new(6, 5)).y, 2);
        assert_eq!(lines(&mut row, (6, 2)), ["abc...", "def..."]);
    }

    #[test]
    fn fixed_label_width_with_a_view_label() {
        let row = |align| {
            Row::from_views(TextView::new("ab").h_align(align), TextView::new("cd")).label_width(6)
        };
        assert_eq!(
            row(HAlign::Left).required_size(Vec2::new(50, 5)),
            Vec2::new(9, 1)
        );
        assert_eq!(lines(&mut row(HAlign::Left), (9, 1)), ["ab.....cd"]);
        // The view aligns itself in the whole column.
        assert_eq!(lines(&mut row(HAlign::Right), (9, 1)), ["....ab.cd"]);
        assert_eq!(lines(&mut row(HAlign::Center), (9, 1)), ["..ab...cd"]);

        // `label_align` only applies to text labels.
        let mut with_align = row(HAlign::Left).label_align(HAlign::Right);
        assert_eq!(lines(&mut with_align, (9, 1)), ["ab.....cd"]);
    }

    #[test]
    fn view_label_is_cropped() {
        let mut row = Row::from_views(TextView::new("abcdef").no_wrap(), TextView::new("xy"));
        assert_eq!(lines(&mut row, (4, 1)), ["abcd"]);
        assert_eq!(lines(&mut row, (6, 1)), ["abcdef"]);
        assert_eq!(lines(&mut row, (7, 1)), ["abcdef."]);
        assert_eq!(lines(&mut row, (8, 1)), ["abcdef.x"]);
        assert_eq!(lines(&mut row, (0, 1)), [""]);

        let mut row =
            Row::from_views(TextView::new("abcdef").no_wrap(), TextView::new("xy")).label_width(3);
        assert_eq!(lines(&mut row, (8, 1)), ["abc.xy.."]);
        assert_eq!(lines(&mut row, (2, 1)), ["ab"]);
    }

    #[test]
    fn call_on_any_reaches_both_views() {
        let mut row = Row::from_views(
            EditView::new().with_name("label"),
            EditView::new().with_name("value"),
        );
        assert!(
            row.call_on_name("label", |v: &mut EditView| v.set_content("l"))
                .is_some()
        );
        assert!(
            row.call_on_name("value", |v: &mut EditView| v.set_content("v"))
                .is_some()
        );
        assert!(row.call_on_name("other", |_: &mut EditView| ()).is_none());

        // Only the value can be focused.
        assert!(row.focus_view(&Selector::Name("value")).is_ok());
        assert!(row.focus_view(&Selector::Name("label")).is_err());
    }

    #[test]
    fn label_gets_no_focus_and_no_events() {
        let (label_clicks, value_clicks) =
            (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let button = |clicks: &Arc<AtomicUsize>| {
            let clicks = clicks.clone();
            Button::new("B", move |_| {
                clicks.fetch_add(1, Ordering::Relaxed);
            })
        };
        let mut row = Row::from_views(button(&label_clicks), button(&value_clicks));
        let size = row.required_size(Vec2::new(40, 5));
        row.layout(size);
        let mut cursive = crate::Cursive::new();

        assert!(row.take_focus(crate::direction::Direction::none()).is_ok());
        let res = row.on_event(Event::Key(crate::event::Key::Enter));
        assert!(res.is_consumed());
        res.process(&mut cursive);
        assert_eq!(
            (
                label_clicks.load(Ordering::Relaxed),
                value_clicks.load(Ordering::Relaxed)
            ),
            (0, 1)
        );

        // A click on the label is not given to anyone.
        let click = |x| Event::Mouse {
            offset: Vec2::zero(),
            position: Vec2::new(x, 0),
            event: MouseEvent::Release(MouseButton::Left),
        };
        row.on_event(click(1)).process(&mut cursive);
        assert_eq!(label_clicks.load(Ordering::Relaxed), 0);

        // A focusable label with an inert value: nothing to focus.
        let mut row = Row::from_views(button(&label_clicks), TextView::new("v"));
        assert!(row.take_focus(crate::direction::Direction::none()).is_err());
        row.layout(Vec2::new(10, 1));
        assert!(
            !row.on_event(Event::Key(crate::event::Key::Enter))
                .is_consumed()
        );
        assert_eq!(label_clicks.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn each_child_is_measured_once() {
        let probe = |x, y| {
            let calls = Arc::new(AtomicUsize::new(0));
            let view = Probe {
                size: Vec2::new(x, y),
                calls: calls.clone(),
            };
            (view, calls)
        };
        let count = |calls: &Arc<AtomicUsize>| calls.load(Ordering::Relaxed);

        let (label, label_calls) = probe(3, 2);
        let (value, value_calls) = probe(4, 1);
        let mut row = Row::from_views(label, value);
        assert_eq!(row.required_size(Vec2::new(40, 10)), Vec2::new(8, 2));
        assert_eq!((count(&label_calls), count(&value_calls)), (1, 1));

        // Also with a fixed label width.
        let (label, label_calls) = probe(3, 2);
        let (value, value_calls) = probe(4, 1);
        let mut row = Row::from_views(label, value).label_width(5);
        assert_eq!(row.required_size(Vec2::new(40, 10)), Vec2::new(10, 2));
        assert_eq!((count(&label_calls), count(&value_calls)), (1, 1));
    }

    #[test]
    fn replacing_the_label() {
        let mut row = Row::new("ab", TextView::new("cd"));
        let key = row.layout_key();
        row.set_label_view(TextView::new("ab"));
        assert_ne!(row.layout_key(), key);
        assert!(row.needs_relayout());
        // A view label is not aligned by the row.
        row.set_label_width(5);
        row.set_label_align(HAlign::Right);
        assert_eq!(lines(&mut row, (8, 1)), ["ab....cd"]);
        // Back to text.
        row.set_label("ab");
        assert_eq!(lines(&mut row, (8, 1)), ["...ab.cd"]);
        // The label can be changed in place.
        row.get_label_view_mut()
            .downcast_mut::<TextView>()
            .unwrap()
            .set_content("xyz");
        assert_eq!(lines(&mut row, (9, 1)), ["..xyz.cd."]);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config_with_a_view_label() {
        let config = r#"
            Row:
                label:
                    TextView:
                        content: ab
                value:
                    TextView:
                        content: cd
                label_width: 4
                label_align: right
        "#;
        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let mut view = crate::builder::Context::new().build(&config).unwrap();
        let row = view.downcast_mut::<Row>().unwrap();
        assert!(!row.text_label);
        assert!(row.get_label_view().downcast_ref::<TextView>().is_some());
        // The text view aligns itself, and ignores `label_align`.
        assert_eq!(lines(row, (7, 1)), ["ab...cd"]);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config() {
        let config = r#"
            Row:
                label: Latency
                value:
                    TextView:
                        content: 22ms
                gap: 2
                label_width: 9
                label_align: right
        "#;
        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let mut view = crate::builder::Context::new().build(&config).unwrap();
        let row = view
            .downcast_mut::<Row>()
            .expect("the view should be a Row");
        assert_eq!(label_source(row), "Latency");
        assert_eq!(row.gap, 2);
        assert_eq!(row.label_width, Some(9));
        assert_eq!(row.label_align, HAlign::Right);
        assert_eq!(lines(row, (16, 1)), ["..Latency..22ms."]);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config_defaults() {
        let config: crate::builder::Config =
            yaml_serde::from_str("Row:\n  label: a\n  value:\n    TextView:\n      content: b\n")
                .unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let row = view.downcast_ref::<Row>().unwrap();
        assert_eq!(row.gap, 1);
        assert_eq!(row.label_width, None);
        assert_eq!(row.label_align, HAlign::Left);
    }

    /// A parent that gives the row exactly the size it asked for must get the value drawn.
    #[test]
    fn value_survives_getting_exactly_the_required_size() {
        let mut row = Row::from_views(TextView::new("aaaa bbbbbb"), TextView::new("vv").no_wrap());
        let size = row.required_size(Vec2::new(9, 10));
        let drawn = lines(&mut row, size);
        assert!(
            drawn.iter().any(|l| l.contains("vv")),
            "required {size:?} but the value was not drawn: {drawn:?}"
        );
    }

    /// The label column used by `layout` is the one `required_size` accounted for.
    #[test]
    fn layout_keeps_the_column_that_required_size_measured() {
        for text in ["aaa bbb ccc ddd", "aaaa bbbbbb", "a b c d e f g h"] {
            for req_w in 9..30 {
                let mut row = Row::from_views(TextView::new(text), TextView::new("vv").no_wrap());
                let size = row.required_size(Vec2::new(req_w, 10));
                row.layout(size);
                // The value is 2 wide, after the gap.
                let value = size.x.saturating_sub(row.column + 1);
                assert_eq!(
                    value, 2,
                    "{text:?} asked {req_w} got {size:?}: label column {}",
                    row.column
                );
                // The label has the lines the row reserved for it.
                let label = row.label.required_size(Vec2::new(row.column, size.y));
                assert!(
                    label.y <= size.y,
                    "{text:?} asked {req_w}: {label:?} in {size:?}"
                );
            }
        }
    }

    #[test]
    fn size_asked_before_other_sizes_still_gets_its_column() {
        // A parent can measure the row under several constraints, and then
        // lay it out with any of the answers.
        let mut row = Row::from_views(TextView::new("aaaa bbbbbb"), TextView::new("vv").no_wrap());
        let narrow = row.required_size(Vec2::new(9, 10));
        let wide = row.required_size(Vec2::new(40, 10));
        assert_ne!(narrow, wide);
        for size in [narrow, wide, narrow] {
            row.layout(size);
            assert!(
                lines(&mut row, size).iter().any(|l| l.contains("vv")),
                "{size:?}"
            );
        }
        // Sizes that were not asked for are measured again, as before.
        assert_eq!(lines(&mut row, (14, 1)), ["aaaa.bbbbbb.vv"]);
    }

    #[test]
    fn a_live_label_that_changed_is_measured_again() {
        let text = Arc::new(parking_lot::Mutex::new(String::from("aaaa bbbbbb")));
        let source = text.clone();
        let mut row = Row::from_views(
            TextView::live(move || source.lock().clone()),
            TextView::new("vv").no_wrap(),
        );
        let size = row.required_size(Vec2::new(9, 10));
        assert_eq!(size, Vec2::new(12, 2));
        // The answer of `required_size` is stale: the label has a new size.
        *text.lock() = "x".into();
        assert_eq!(lines(&mut row, size)[0], "x.vv........");
    }

    #[test]
    fn layout_is_cached() {
        crate::views::layout_key_tests::assert_layout_is_cached(|| {
            Row::new("cpu", crate::views::Gauge::value(0.5))
        });
        crate::views::layout_key_tests::assert_layout_is_cached(|| Row::text("cpu", "50%"));
    }

    /// `set_label_align` is visible without a fixed width, on a multi-line label.
    #[test]
    fn label_align_aligns_the_lines_of_a_multi_line_label() {
        let natural = |align| {
            let mut row = Row::new("ab\ncde", TextView::new("v")).label_align(align);
            let size = row.required_size(Vec2::new(40, 5));
            lines(&mut row, size)
        };
        assert_eq!(natural(HAlign::Left), ["ab..v", "cde.."]);
        assert_eq!(natural(HAlign::Right), [".ab.v", "cde.."]);
        // A one-line label fills its column: nothing to align.
        let one_line = |align| {
            let mut row = Row::new("abc", TextView::new("v")).label_align(align);
            let size = row.required_size(Vec2::new(40, 5));
            lines(&mut row, size)
        };
        assert_eq!(one_line(HAlign::Left), one_line(HAlign::Right));
    }

    #[test]
    fn degenerate_sizes_stay_inside_and_views_do_not_overlap() {
        let labels = [
            "",
            "L",
            "LLLLLLLL",
            "日本日本",
            "LL\nLLLL",
            "\n",
            "L\n",
            "L\n\nL",
        ];
        let mut problems = Vec::new();
        for label in labels {
            for as_view in [false, true] {
                for label_width in [None, Some(0), Some(1), Some(3), Some(50)] {
                    for gap in [0, 1, 5] {
                        for align in [HAlign::Left, HAlign::Center, HAlign::Right] {
                            for (w, h) in [
                                (0, 0),
                                (1, 1),
                                (2, 1),
                                (3, 2),
                                (6, 1),
                                (9, 3),
                                (0, 2),
                                (4, 0),
                            ] {
                                let r = std::panic::catch_unwind(|| {
                                    let value = TextView::new("V\nV").no_wrap();
                                    let mut row = if as_view {
                                        Row::from_views(TextView::new(label).no_wrap(), value)
                                    } else {
                                        Row::new(label, value)
                                    }
                                    .label_width(label_width)
                                    .gap(gap)
                                    .label_align(align);
                                    let req = row.required_size(Vec2::new(w, h));
                                    let size = Vec2::new(w, h);
                                    row.layout(size);
                                    let mut buffer = PrintBuffer::new();
                                    buffer.resize(Vec2::new(w + 3, h + 3));
                                    let buffer = RwLock::new(buffer);
                                    let theme = Theme::retro();
                                    let printer =
                                        Printer::new(Vec2::new(w + 3, h + 3), &theme, &buffer)
                                            .cropped(size);
                                    row.draw(&printer);
                                    let buffer = buffer.read();
                                    let mut out = Vec::new();
                                    for x in 0..w + 3 {
                                        for y in 0..h + 3 {
                                            let t = buffer.cell_text(Vec2::new(x, y));
                                            let blank = matches!(t, None | Some("") | Some(" "));
                                            if (x >= w || y >= h) && !blank {
                                                out.push(format!("outside {x},{y}={t:?}"));
                                            }
                                            // gap cells and label cells right of the column must not hold the other view
                                            if x < w && y < h {
                                                let col = row.column;
                                                if x >= col && x < col + gap && !blank {
                                                    out.push(format!("gap cell {x},{y}={t:?}"));
                                                }
                                                if x >= col + gap && t == Some("L") {
                                                    out.push(format!("label spill {x},{y}"));
                                                }
                                                if x < col && t == Some("V") {
                                                    out.push(format!("value in label col {x},{y}"));
                                                }
                                            }
                                        }
                                    }
                                    let _ = (req, row.important_area(size));
                                    out
                                });
                                match r {
                                    Err(_) => problems.push(format!(
                                        "PANIC label {label:?} view {as_view} lw {label_width:?} gap {gap} {align:?} size {w}x{h}"
                                    )),
                                    Ok(out) if !out.is_empty() => problems.push(format!(
                                        "label {label:?} view {as_view} lw {label_width:?} gap {gap} {align:?} size {w}x{h}: {out:?}"
                                    )),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
        problems.dedup();
        assert!(
            problems.is_empty(),
            "{} problems, first: {:#?}",
            problems.len(),
            &problems[..problems.len().min(8)]
        );
    }

    /// Doc of `set_label_align`: "no visible effect when the column has the natural width".

    #[test]
    fn zero_width_value_with_common_views() {
        let makers: Vec<(&str, fn() -> Box<dyn View>)> = vec![
            ("EditView", || Box::new(EditView::new().content("hello"))),
            ("Button", || Box::new(Button::new("Ok", |_| ()))),
            ("Checkbox", || Box::new(Checkbox::new())),
            ("ProgressBar", || Box::new(ProgressBar::new())),
            ("Gauge", || Box::new(Gauge::value(0.5))),
            ("ScrollView", || {
                Box::new(ScrollView::new(TextView::new("a b c d e f g")))
            }),
            ("LinearLayout", || {
                Box::new(LinearLayout::horizontal().child(TextView::new("ab")))
            }),
        ];
        let mut failures = Vec::new();
        for (name, make) in makers {
            for width in [0, 1, 3, 6, 7, 8] {
                let result = std::panic::catch_unwind(|| {
                    let mut row = Row::new("abcdef", make());
                    let size = Vec2::new(width, 2);
                    row.layout(size);
                    let mut buffer = PrintBuffer::new();
                    buffer.resize(size);
                    let buffer = RwLock::new(buffer);
                    row.draw(&Printer::new(size, &Theme::retro(), &buffer));
                    let _ = row.take_focus(crate::direction::Direction::none());
                    let _ = row.on_event(Event::Char('x'));
                    let _ = row.on_event(Event::Key(crate::event::Key::Left));
                    let _ = row.on_event(Event::Mouse {
                        offset: Vec2::zero(),
                        position: Vec2::new(width.saturating_sub(1), 0),
                        event: MouseEvent::Press(MouseButton::Left),
                    });
                    let _ = row.important_area(size);
                });
                if result.is_err() {
                    failures.push(format!("{name} at width {width}"));
                }
            }
        }
        assert!(failures.is_empty(), "{failures:?}");
    }

    /// A parent that gives the row exactly the size it asked for must get the
    /// value drawn.

    #[test]
    fn clicks_on_label_and_gap_do_not_reach_the_value() {
        let mut row = Row::new("label", EditView::new().content("hello")).gap(3);
        row.layout(Vec2::new(30, 1));
        row.take_focus(crate::direction::Direction::none()).unwrap();
        let cursor = |row: &Row| {
            row.get_value()
                .downcast_ref::<EditView>()
                .unwrap()
                .get_cursor()
        };
        assert_eq!(cursor(&row), 5);
        // x = 0..5 label, 5..8 gap, 8.. value
        for x in [0, 2, 4, 5, 6, 7] {
            row.on_event(Event::Mouse {
                offset: Vec2::zero(),
                position: Vec2::new(x, 0),
                event: MouseEvent::Press(MouseButton::Left),
            });
            assert_eq!(cursor(&row), 5, "click at {x} moved the cursor");
        }
        // First cell of the value.
        row.on_event(Event::Mouse {
            offset: Vec2::zero(),
            position: Vec2::new(8, 0),
            event: MouseEvent::Press(MouseButton::Left),
        });
        assert_eq!(cursor(&row), 0);
    }

    #[test]
    fn button_click_in_the_gap_and_last_cell() {
        let clicks = Arc::new(AtomicUsize::new(0));
        let c = clicks.clone();
        let mut row = Row::new(
            "label",
            Button::new("Ok", move |_| {
                c.fetch_add(1, Ordering::Relaxed);
            }),
        );
        let size = row.required_size(Vec2::new(40, 5));
        row.layout(size);
        let mut cursive = crate::Cursive::new();
        let mut click = |x: usize| {
            row.on_event(Event::Mouse {
                offset: Vec2::zero(),
                position: Vec2::new(x, 0),
                event: MouseEvent::Release(MouseButton::Left),
            })
            .process(&mut cursive);
        };
        // The gap
        click(5);
        assert_eq!(clicks.load(Ordering::Relaxed), 0);
        // Last cell of the button: size.x - 1
        click(size.x - 1);
        assert_eq!(clicks.load(Ordering::Relaxed), 1);
        // Past the row.
        click(size.x);
        assert_eq!(clicks.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn live_label_changing_size_in_a_linear_layout() {
        let text = Arc::new(parking_lot::Mutex::new(String::from("a")));
        let source = text.clone();
        let mut layout = LinearLayout::vertical()
            .child(Row::from_views(
                TextView::live(move || source.lock().clone()).no_wrap(),
                TextView::new("v").no_wrap(),
            ))
            .child(Row::text("x", "y"));
        for t in ["a", "abcdef", "ab\ncd", "a", "wwwwwwwwwwwwww", "b"] {
            *text.lock() = t.to_string();
            let size = layout.required_size(Vec2::new(40, 10));
            let drawn = lines(&mut layout, size);
            let first_line = t.lines().next().unwrap();
            assert!(drawn[0].starts_with(first_line), "{t:?}: {drawn:?}");
            let widest = t.lines().map(|l| l.len()).max().unwrap();
            assert_eq!(
                drawn[0].chars().count(),
                widest + 2,
                "{t:?}: {drawn:?} {size:?}"
            );
            assert!(drawn[0].ends_with('v'), "{t:?}: {drawn:?}");
        }
    }

    #[test]
    fn live_label_changing_size_in_a_scroll_view() {
        let text = Arc::new(parking_lot::Mutex::new(String::from("a")));
        let source = text.clone();
        let mut view = ScrollView::new(LinearLayout::vertical().child(Row::from_views(
            TextView::live(move || source.lock().clone()).no_wrap(),
            TextView::new("v").no_wrap(),
        )));
        for t in ["a", "abcdef", "ab\ncd", "a", "wwwwwwwwwwwwww", "b"] {
            *text.lock() = t.to_string();
            let size = Vec2::new(30, 6);
            let drawn = lines(&mut view, size);
            let first_line = t.lines().next().unwrap();
            assert!(drawn[0].starts_with(first_line), "{t:?}: {drawn:?}");
            assert!(drawn[0].contains('v'), "{t:?}: {drawn:?}");
        }
    }
}
