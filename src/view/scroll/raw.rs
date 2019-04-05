use crate::event::{Event, EventResult};
use crate::rect::Rect;
use crate::view::scroll;
use crate::xy::XY;
use crate::Printer;
use crate::Vec2;

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

fn sizes_when_scrolling<Model, GetScroller, RequiredSize>(
    constraint: Vec2,
    scrollable: XY<bool>,
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
    let scrollbar_size = scrollable.swap().select_or(
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
        inner_size + scrollbar_size,
    );

    // In strict mode, there's no way our size is over constraints.
    let size = if strict {
        size.or_min(constraint)
    } else {
        size
    };

    // On non-scrolling axis, give inner_size the available space instead.
    let inner_size = get_scroller(model)
        .is_enabled()
        .select_or(inner_size, size.saturating_sub(scrollbar_size));

    let new_scrollable = inner_size.zip_map(size, |i, s| i > s);

    (inner_size, size, new_scrollable)
}

fn sizes<Model, GetScroller, RequiredSize>(
    constraint: Vec2,
    strict: bool,
    needs_relayout: bool,
    model: &mut Model,
    get_scroller: &mut GetScroller,
    required_size: &mut RequiredSize,
) -> (Vec2, Vec2)
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
    let (inner_size, size, scrollable) = sizes_when_scrolling(
        constraint,
        XY::new(false, false),
        strict,
        model,
        get_scroller,
        required_size,
    );

    // If we need to add scrollbars, the available size will change.
    if scrollable.any() && get_scroller(model).get_show_scrollbars() {
        // Attempt 2: he wants to scroll? Sure!
        // Try again with some space for the scrollbar.
        let (inner_size, size, new_scrollable) = sizes_when_scrolling(
            constraint,
            scrollable,
            strict,
            model,
            get_scroller,
            required_size,
        );
        if scrollable == new_scrollable {
            // Yup, scrolling did it. We're good to go now.
            (inner_size, size)
        } else {
            // Again? We're now scrolling in a new direction?
            // There is no end to this!
            let (inner_size, size, _) = sizes_when_scrolling(
                constraint,
                new_scrollable,
                strict,
                model,
                get_scroller,
                required_size,
            );

            // That's enough. If the inner view changed again, ignore it!
            // That'll teach it.
            (inner_size, size)
        }
    } else {
        // We're not showing any scrollbar, either because we don't scroll
        // or because scrollbars are hidden.
        (inner_size, size)
    }
}

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
    get_scroller(model).set_last_size(size);

    // This is what we'd like
    let (inner_size, self_size) = sizes(
        size,
        true,
        needs_relayout,
        model,
        &mut get_scroller,
        &mut required_size,
    );

    get_scroller(model).set_inner_size(inner_size);
    get_scroller(model).build_cache(self_size, size);

    layout(model, inner_size);

    get_scroller(model).update_offset();
}

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
    let (_, size) = sizes(
        constraint,
        false,
        needs_relayout,
        model,
        &mut get_scroller,
        &mut required_size,
    );

    size
}

pub fn on_event<Model, GetScroller, OnEvent, ImportantArea>(
    event: Event,
    model: &mut Model,
    mut get_scroller: GetScroller,
    mut on_event: OnEvent,
    mut important_area: ImportantArea,
) -> EventResult
where
    Model: ?Sized,
    GetScroller: FnMut(&mut Model) -> &mut scroll::Core,
    OnEvent: FnMut(&mut Model, Event) -> EventResult,
    ImportantArea: FnMut(&Model, Vec2) -> Rect,
{
    let mut relative_event = event.clone();
    let inside = get_scroller(model).is_event_inside(&mut relative_event);
    let result = if inside {
        on_event(model, relative_event)
    } else {
        EventResult::Ignored
    };
    let inner_size = get_scroller(model).inner_size();
    let important = important_area(model, inner_size);
    get_scroller(model).on_inner_event(event, result, important)
}
