use crate::vec::Vec2;
use crate::Printer;

use crate::direction::Direction;
use crate::event::{Event, EventResult};
use crate::view::scroll::ScrollCore;
use crate::view::scroll::{InnerLayout, InnerOnEvent, InnerRequiredSize};

/// Provide scrolling functionalities to a view.
///
/// You're not supposed to use this directly,
/// but it can be helpful if you create your own Views.
#[derive(Default, Debug)]
pub struct ScrollBase {
    core: ScrollCore,
}

struct RequiredSize<F>(F);

impl<F> InnerRequiredSize for RequiredSize<F>
where
    F: FnMut(Vec2) -> Vec2,
{
    fn needs_relayout(&self) -> bool {
        true
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.0(constraint)
    }
}

impl<F> InnerLayout for RequiredSize<F>
where
    F: FnMut(Vec2) -> Vec2,
{
    fn layout(&mut self, _size: Vec2) {}

    fn needs_relayout(&self) -> bool {
        true
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.0(constraint)
    }
}

impl ScrollBase {
    /// Creates a new, uninitialized scrollbar.
    pub fn new() -> Self {
        ScrollBase {
            core: ScrollCore::new(),
        }
    }

    /// Performs `View::layout()`.
    pub fn layout<F>(&mut self, size: Vec2, required_size: F)
    where
        F: FnMut(Vec2) -> Vec2,
    {
        self.core.layout(size, RequiredSize(required_size));
    }

    /// Performs `View::required_size()`.
    pub fn required_size<F>(
        &mut self, constraint: Vec2, required_size: F,
    ) -> Vec2
    where
        F: FnMut(Vec2) -> Vec2,
    {
        self.core
            .required_size(constraint, RequiredSize(required_size))
    }

    /// Draws the scroll bar and the content using the given drawer.
    ///
    /// `line_drawer` will be called once for each line that needs to be drawn.
    /// It will be given the absolute ID of the item to draw..
    /// It will also be given a printer with the correct offset,
    /// so it should only print on the first line.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::view::ScrollBase;
    /// # use cursive::Printer;
    /// # use cursive::theme;
    /// # use cursive::backend;
    /// # let scrollbase = ScrollBase::new();
    /// # let b = backend::dummy::Backend::init();
    /// # let t = theme::load_default();
    /// # let printer = Printer::new((5,1), &t, &*b);
    /// # let printer = &printer;
    /// let lines = ["Line 1", "Line number 2"];
    /// scrollbase.draw(printer, |printer, i| {
    ///     printer.print((0,0), lines[i]);
    /// });
    /// ```
    pub fn draw<F>(&self, printer: &Printer<'_, '_>, mut line_drawer: F)
    where
        F: FnMut(&Printer<'_, '_>, usize),
    {
        self.core.draw(printer, |printer| {
            let start = printer.content_offset.y;
            let end = start + printer.output_size.y;
            for y in start..end {
                let printer =
                    printer.offset((0, y)).cropped((printer.size.x, 1));
                line_drawer(&printer, y);
            }
        });
    }

    /// Performs `View::take_focus()`.
    pub fn take_focus<F>(
        &mut self, source: Direction, inner_take_focus: F,
    ) -> bool
    where
        F: FnOnce(Direction) -> bool,
    {
        self.core.take_focus(source, inner_take_focus)
    }

    /// Performs `View::on_event()`.
    pub fn on_event<I>(&mut self, event: Event, inner: I) -> EventResult
    where
        I: InnerOnEvent,
    {
        self.core.on_event(event, inner)
    }
}
