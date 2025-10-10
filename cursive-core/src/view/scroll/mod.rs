//! Core mechanisms to implement scrolling.
//!
//! *This module is still unstable and may go through breaking changes.*
//!
//! This modules defines:
//!
//! * [`scroll::Core`](crate::view::scroll::Core): stores the state variables
//!   required to handle scrolling. Any view that needs to implement scrolling
//!   should embed a `scroll::Core`.
//!     * [`scroll::Scroller`](crate::view::scroll::Scroller): a trait for
//!       something that embeds a such a `scroll::Core`.
//! * Some free functions to help implement the usual `View` trait for a type
//!   implementing `scroll::Scroller`.
//!   Some methods, like `View::call_on_any`, are not affected by scrolling
//!   and are not covered here.
//!     * The functions defined here will usually take a reference to the
//!       `Scroller` object, as well as closures to implement the "inner view".
//!
//! [`ScrollView`](crate::views::ScrollView) may be an easier way to add scrolling to an existing view.

#[macro_use]
mod core;
mod raw;

pub use self::core::{Core, Scroller};

use crate::event::{Event, EventResult};
use crate::{Printer, Rect, Vec2};

/// Defines the scrolling behaviour on content or size change
#[derive(Debug)]
pub enum ScrollStrategy {
    /// Keeps the same row number
    KeepRow,
    /// Sticks to the top.
    StickToTop,
    /// Sticks to the bottom of the view.
    StickToBottom,
}

impl Default for ScrollStrategy {
    fn default() -> Self {
        ScrollStrategy::KeepRow
    }
}

impl std::str::FromStr for ScrollStrategy {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "KeepRow" | "keep_row" => Self::KeepRow,
            "StickToTop" | "stick_to_top" => Self::StickToTop,
            "StickToBottom" | "stick_to_bottom" => Self::StickToBottom,
            _ => return Err(()),
        })
    }
}

/// Performs `View::on_event` on a `scroll::Scroller`.
///
/// Example:
///
/// ```
/// use cursive_core::event::{Event, EventResult};
/// use cursive_core::view::{scroll, View};
/// use cursive_core::{Printer, Rect, Vec2};
///
/// struct MyView {
///     core: scroll::Core,
/// }
///
/// cursive_core::impl_scroller!(MyView::core);
///
/// impl MyView {
///     fn inner_on_event(&mut self, event: Event) -> EventResult {
///         EventResult::Ignored
///     }
///
///     fn inner_important_area(&self, size: Vec2) -> Rect {
///         Rect::from_size((0, 0), size)
///     }
/// }
///
/// impl View for MyView {
/// # fn draw(&self, printer: &Printer) {}
///     fn on_event(&mut self, event: Event) -> EventResult {
///         scroll::on_event(
///             self,
///             event,
///             Self::inner_on_event,
///             Self::inner_important_area,
///         )
///     }
/// }
/// ```
pub fn on_event<T, OnEvent, ImportantArea>(
    scroller: &mut T,
    event: Event,
    on_event: OnEvent,
    important_area: ImportantArea,
) -> EventResult
where
    T: Scroller,
    OnEvent: FnMut(&mut T, Event) -> EventResult,
    ImportantArea: FnMut(&T, Vec2) -> Rect,
{
    raw::on_event(
        event,
        scroller,
        Scroller::get_scroller_mut,
        on_event,
        important_area,
    )
}

/// Performs `View::important_area` on a `scroll::Scroller`.
pub fn important_area<T, ImportantArea>(
    scroller: &T,
    size: Vec2,
    mut important_area: ImportantArea,
) -> Rect
where
    T: Scroller,
    ImportantArea: FnMut(&T, Vec2) -> Rect,
{
    let viewport = scroller.get_scroller().content_viewport();
    let area = important_area(scroller, size);
    let top_left = area.top_left().saturating_sub(viewport.top_left());
    let bot_right = area
        .bottom_right()
        .saturating_sub(viewport.top_left())
        .or_min(viewport.bottom_right());

    Rect::from_corners(top_left, bot_right)
}

/// Performs `View::layout` on a `scroll::Scroller`.
pub fn layout<T, Layout, RequiredSize>(
    scroller: &mut T,
    size: Vec2,
    needs_relayout: bool,
    layout: Layout,
    required_size: RequiredSize,
) where
    T: Scroller,
    Layout: FnMut(&mut T, Vec2),
    RequiredSize: FnMut(&mut T, Vec2) -> Vec2,
{
    raw::layout(
        size,
        needs_relayout,
        scroller,
        Scroller::get_scroller_mut,
        required_size,
        layout,
    );
}

/// Performs `View::required_size` on a `scroll::Scroller`.
pub fn required_size<T, RequiredSize>(
    scroller: &mut T,
    size: Vec2,
    needs_relayout: bool,
    required_size: RequiredSize,
) -> Vec2
where
    T: Scroller,
    RequiredSize: FnMut(&mut T, Vec2) -> Vec2,
{
    raw::required_size(
        size,
        needs_relayout,
        scroller,
        Scroller::get_scroller_mut,
        required_size,
    )
}

/// Performs `View::draw` on a `scroll::Scroller`.
pub fn draw<T, Draw>(scroller: &T, printer: &Printer, draw: Draw)
where
    T: Scroller,
    Draw: FnOnce(&T, &Printer),
{
    raw::draw(printer, scroller, Scroller::get_scroller, draw);
}

/// Performs a line-based `View::draw` on a `scroll::Scroller`.
///
/// This is an alternative to `scroll::draw()` when you just need to print individual lines.
pub fn draw_lines<T, LineDrawer>(scroller: &T, printer: &Printer, mut line_drawer: LineDrawer)
where
    T: Scroller,
    LineDrawer: FnMut(&T, &Printer, usize),
{
    draw(scroller, printer, |s, printer| {
        let start = printer.content_offset.y;
        let end = start + printer.output_size.y;
        for y in start..end {
            let printer = printer.offset((0, y)).cropped((printer.size.x, 1));
            line_drawer(s, &printer, y);
        }
    });
}

/// Draws a frame around the scrollable content.
///
/// `left_border` will be called for each row to draw the left border for the given line number.
pub fn draw_frame<T, LeftBorder, TopBorder, RightBorder, BottomBorder>(
    scroller: &T,
    printer: &Printer,
    mut left_border: LeftBorder,
    mut top_border: TopBorder,
    mut right_border: RightBorder,
    mut bottom_border: BottomBorder,
) where
    T: Scroller,
    LeftBorder: FnMut(&T, &Printer, usize),
    TopBorder: FnMut(&T, &Printer, usize),
    RightBorder: FnMut(&T, &Printer, usize),
    BottomBorder: FnMut(&T, &Printer, usize),
{
    let viewport = scroller.get_scroller().content_viewport();
    let size = printer.size.saturating_sub((1, 1));

    for (i, x) in (viewport.left()..=viewport.right()).enumerate() {
        top_border(scroller, &printer.offset((i + 1, 0)), x);
        bottom_border(scroller, &printer.offset((i + 1, size.y)), x);
    }

    // Also draw padding
    let scrollbar_size = scroller.get_scroller().scrollbar_size();
    printer.print_hline((viewport.right() + 2, 0), scrollbar_size.x, "─");
    printer.print_hline((viewport.right() + 2, size.y), scrollbar_size.x, "─");
    printer.print_vline((0, viewport.bottom() + 2), scrollbar_size.y, "│");
    printer.print_vline((size.x, viewport.bottom() + 2), scrollbar_size.y, "│");

    for (i, y) in (viewport.top()..=viewport.bottom()).enumerate() {
        left_border(scroller, &printer.offset((0, i + 1)), y);
        right_border(scroller, &printer.offset((size.x, i + 1)), y);
    }

    printer.print((0, 0), "┌");
    printer.print(size.keep_y(), "└");
    printer.print(size.keep_x(), "┐");
    printer.print(size, "┘");
}

/// Draws a box-style frame around a scrollable content.
///
/// Assumes horizontal lines are present in the content whenever `is_h_delim`
/// returns `true` (and vertical lines when `is_v_delim` returns `true`).
///
/// It will print a box with the appropriate `├`, `┤` and so on.
pub fn draw_box_frame<T, IsHDelim, IsVDelim>(
    scroller: &T,
    printer: &Printer,
    is_h_delim: IsHDelim,
    is_v_delim: IsVDelim,
) where
    T: Scroller,
    IsHDelim: Fn(&T, usize) -> bool,
    IsVDelim: Fn(&T, usize) -> bool,
{
    draw_frame(
        scroller,
        printer,
        |s, printer, y| {
            if is_h_delim(s, y) {
                printer.print((0, 0), "├");
            } else {
                printer.print((0, 0), "│");
            }
        },
        |s, printer, x| {
            if is_v_delim(s, x) {
                printer.print((0, 0), "┬");
            } else {
                printer.print((0, 0), "─");
            }
        },
        |s, printer, y| {
            if is_h_delim(s, y) {
                printer.print((0, 0), "┤");
            } else {
                printer.print((0, 0), "│");
            }
        },
        |s, printer, x| {
            if is_v_delim(s, x) {
                printer.print((0, 0), "┴");
            } else {
                printer.print((0, 0), "─");
            }
        },
    );
}
