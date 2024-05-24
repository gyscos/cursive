//! Low-level implementation of the `View` trait using a `scroll::Core`.
//!
//! Most functions take a generic `Model` class, and various closures to get
//! the required things from this model.
use crate::{
    event::{Event, EventResult, Key, MouseButton, MouseEvent},
    rect::Rect,
    view::scroll,
    xy::XY,
    Printer, Vec2,
};

/// Implements `View::draw` over the `model`.
pub fn draw<Model, GetScroller, Draw>(
    printer: &Printer,
    model: &Model,
    mut get_scroller: GetScroller,
    inner_draw: Draw,
) where
    Model: ?Sized,
    GetScroller: FnMut(&Model) -> &scroll::Core,
    Draw: FnOnce(&Model, &Printer),
{
    let printer = get_scroller(model).sub_printer(printer);
    inner_draw(model, &printer);
}

/// Intermediate method to get the size requirements of a view.
///
/// Assumes we are already scrolling on the axis designated by `scrolling`.
///
/// `strict` means the result will never be bigger than the constraint.
///
/// Returns (Inner size, Outer size, New scrolling)
fn sizes_when_scrolling<Model, GetScroller, RequiredSize>(
    constraint: Vec2,
    scrolling: XY<bool>,
    strict: bool,
    model: &mut Model,
    get_scroller: &mut GetScroller,
    required_size: &mut RequiredSize,
) -> (Vec2, Vec2, XY<bool>)
where
    Model: ?Sized,
    GetScroller: FnMut(&mut Model) -> &mut scroll::Core,
    RequiredSize: FnMut(&mut Model, Vec2) -> Vec2,
{
    // This is the size taken by the scrollbars.
    let scrollbar_size = scrolling.swap().select_or(
        get_scroller(model).get_scrollbar_padding() + (1, 1),
        Vec2::zero(),
    );

    let available = constraint.saturating_sub(scrollbar_size);

    // This the ideal size for the child. May not be what he gets.
    let inner_size = required_size(model, available);

    // Where we're "enabled", accept the constraints.
    // Where we're not, just forward inner_size.
    let size = get_scroller(model).is_enabled().select_or(
        Vec2::min(inner_size + scrollbar_size, constraint),
        // In strict mode, fill all the space we can:
        match strict {
            true => constraint,
            false => inner_size + scrollbar_size,
        },
    );

    // In strict mode, there's no way our size is over constraints.
    let size = if strict {
        size.or_min(constraint)
    } else {
        size
    };

    // Re-define `available` using the new, actual size.
    let available = size.saturating_sub(scrollbar_size);

    // On non-scrolling axis, give inner_size the available space instead.
    let inner_size = get_scroller(model)
        .is_enabled()
        .select_or(inner_size, available);

    let new_scrolling = inner_size.zip_map(available, |i, s| i > s);

    (inner_size, size, new_scrolling)
}

/// Returns the size requirement of the view.
///
/// Returns (Inner size, Outer size)
fn sizes<Model, GetScroller, RequiredSize>(
    constraint: Vec2,
    strict: bool,
    needs_relayout: bool,
    model: &mut Model,
    get_scroller: &mut GetScroller,
    required_size: &mut RequiredSize,
) -> (Vec2, Vec2, XY<bool>)
where
    Model: ?Sized,
    GetScroller: FnMut(&mut Model) -> &mut scroll::Core,
    RequiredSize: FnMut(&mut Model, Vec2) -> Vec2,
{
    if !needs_relayout {
        if let Some(cached) = get_scroller(model).try_cache(constraint) {
            return cached;
        }
    }

    // Attempt 1: try without scrollbars
    let (inner_size, size, scrolling) = sizes_when_scrolling(
        constraint,
        XY::new(false, false),
        strict,
        model,
        get_scroller,
        required_size,
    );

    // If we need to add scrollbars, the available size will change.
    if scrolling.any() && get_scroller(model).get_show_scrollbars() {
        // Attempt 2: he wants to scroll? Sure!
        // Try again with some space for the scrollbar.
        let (inner_size, size, new_scrolling) = sizes_when_scrolling(
            constraint,
            scrolling,
            strict,
            model,
            get_scroller,
            required_size,
        );
        if scrolling == new_scrolling {
            // Yup, scrolling did it. We're good to go now.
            (inner_size, size, scrolling)
        } else {
            // Again? We're now scrolling in a new direction?
            // There is no end to this!
            let (inner_size, size, _) = sizes_when_scrolling(
                constraint,
                new_scrolling,
                strict,
                model,
                get_scroller,
                required_size,
            );

            // That's enough. If the inner view changed again, ignore it!
            // That'll teach it.
            (inner_size, size, new_scrolling)
        }
    } else {
        // We're not showing any scrollbar, either because we don't scroll
        // or because scrollbars are hidden.
        (inner_size, size, scrolling)
    }
}

/// Implements `View::layout` on the given model.
pub fn layout<Model, GetScroller, RequiredSize, Layout>(
    size: Vec2,
    needs_relayout: bool,
    model: &mut Model,
    mut get_scroller: GetScroller,
    mut required_size: RequiredSize,
    mut layout: Layout,
) where
    Model: ?Sized,
    GetScroller: FnMut(&mut Model) -> &mut scroll::Core,
    RequiredSize: FnMut(&mut Model, Vec2) -> Vec2,
    Layout: FnMut(&mut Model, Vec2),
{
    // This is what we'd like
    let (inner_size, self_size, scrolling) = sizes(
        size,
        true,
        needs_relayout,
        model,
        &mut get_scroller,
        &mut required_size,
    );
    get_scroller(model).set_last_size(self_size, scrolling);
    get_scroller(model).set_inner_size(inner_size);
    get_scroller(model).build_cache(self_size, size, scrolling);

    layout(model, inner_size);

    get_scroller(model).update_offset();
}

/// Implements `View::required_size` on the given model.
pub fn required_size<Model, GetScroller, RequiredSize>(
    constraint: Vec2,
    needs_relayout: bool,
    model: &mut Model,
    mut get_scroller: GetScroller,
    mut required_size: RequiredSize,
) -> Vec2
where
    Model: ?Sized,
    GetScroller: FnMut(&mut Model) -> &mut scroll::Core,
    RequiredSize: FnMut(&mut Model, Vec2) -> Vec2,
{
    let (_, size, _) = sizes(
        constraint,
        false,
        needs_relayout,
        model,
        &mut get_scroller,
        &mut required_size,
    );

    size
}

/// Implements `View::on_event` on the given model.
pub fn on_event<Model: ?Sized>(
    event: Event,
    model: &mut Model,
    mut get_scroller: impl FnMut(&mut Model) -> &mut scroll::Core,
    mut on_event: impl FnMut(&mut Model, Event) -> EventResult,
    mut important_area: impl FnMut(&Model, Vec2) -> Rect,
) -> EventResult {
    let mut relative_event = event.clone();
    let inside = get_scroller(model).is_event_inside(&mut relative_event);
    let result = if inside {
        on_event(model, relative_event)
    } else {
        EventResult::Ignored
    };

    match result {
        EventResult::Ignored => {
            // The view ignored the event, so we're free to use it.

            // If it's an arrow, try to scroll in the given direction.
            // If it's a mouse scroll, try to scroll as well.
            // Also allow Ctrl+arrow to move the view,
            // without affecting the selection.
            match event {
                Event::Mouse {
                    event: MouseEvent::WheelUp,
                    ..
                } if get_scroller(model).can_scroll_up() => {
                    get_scroller(model).scroll_up(3);
                }
                Event::Mouse {
                    event: MouseEvent::WheelDown,
                    ..
                } if get_scroller(model).can_scroll_down() => {
                    get_scroller(model).scroll_down(3);
                }
                Event::Mouse {
                    event: MouseEvent::Press(MouseButton::Left),
                    position,
                    offset,
                } if get_scroller(model).get_show_scrollbars()
                    && position
                        .checked_sub(offset)
                        .map(|position| get_scroller(model).start_drag(position))
                        .unwrap_or(false) =>
                {
                    // Just consume the event.
                }
                Event::Mouse {
                    event: MouseEvent::Hold(MouseButton::Left),
                    position,
                    offset,
                } if get_scroller(model).get_show_scrollbars() => {
                    let position = position.saturating_sub(offset);
                    get_scroller(model).drag(position);
                }
                Event::Mouse {
                    event: MouseEvent::Release(MouseButton::Left),
                    ..
                } => {
                    get_scroller(model).release_grab();
                }
                Event::Key(Key::Home) if get_scroller(model).is_enabled().any() => {
                    let actions: XY<fn(&mut scroll::Core)> =
                        XY::new(scroll::Core::scroll_to_left, scroll::Core::scroll_to_top);
                    let scroller = get_scroller(model);
                    actions.run_if(scroller.is_enabled(), |a| a(scroller));
                }
                Event::Key(Key::End) if get_scroller(model).is_enabled().any() => {
                    let actions: XY<fn(&mut scroll::Core)> = XY::new(
                        scroll::Core::scroll_to_right,
                        scroll::Core::scroll_to_bottom,
                    );
                    let scroller = get_scroller(model);
                    actions.run_if(scroller.is_enabled(), |a| a(scroller));
                }
                Event::Ctrl(Key::Up) | Event::Key(Key::Up)
                    if get_scroller(model).can_scroll_up() =>
                {
                    get_scroller(model).scroll_up(1);
                }
                Event::Key(Key::PageUp) if get_scroller(model).can_scroll_up() => {
                    let scroller = get_scroller(model);
                    scroller.scroll_up(scroller.last_available_size().y);
                }
                Event::Key(Key::PageDown) if get_scroller(model).can_scroll_down() => {
                    // No `min` check here - we allow going over the edge.
                    let scroller = get_scroller(model);
                    scroller.scroll_down(scroller.last_available_size().y);
                }
                Event::Ctrl(Key::Down) | Event::Key(Key::Down)
                    if get_scroller(model).can_scroll_down() =>
                {
                    get_scroller(model).scroll_down(1);
                }
                Event::Ctrl(Key::Left) | Event::Key(Key::Left)
                    if get_scroller(model).can_scroll_left() =>
                {
                    let scroller = get_scroller(model);
                    scroller.scroll_left(scroller.last_available_size().x);
                }
                Event::Ctrl(Key::Right) | Event::Key(Key::Right)
                    if get_scroller(model).can_scroll_right() =>
                {
                    let scroller = get_scroller(model);
                    scroller.scroll_right(scroller.last_available_size().x);
                }
                _ => return EventResult::Ignored,
            };

            // We just scrolled manually, so reset the scroll strategy.
            get_scroller(model).set_scroll_strategy(scroll::ScrollStrategy::KeepRow);

            // TODO: return callback on_scroll?
            EventResult::Consumed(None)
        }
        other => {
            // The view consumed the event. Maybe something changed?
            let inner_size = get_scroller(model).inner_size();
            let important = important_area(model, inner_size);
            get_scroller(model).scroll_to_rect(important);

            other
        }
    }
}
