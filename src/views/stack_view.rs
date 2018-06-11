use direction::Direction;
use event::{Event, EventResult};
use std::any::Any;
use std::cell;
use std::ops::Deref;
use theme::ColorStyle;
use vec::Vec2;
use view::{IntoBoxedView, Offset, Position, Selector, View, ViewWrapper};
use views::{Layer, ShadowView, ViewBox};
use Printer;
use With;

/// Simple stack of views.
/// Only the top-most view is active and can receive input.
pub struct StackView {
    // Store layers from back to front.
    layers: Vec<Child>,
    last_size: Vec2,
    // Flag indicates if undrawn areas of the background are exposed
    // and therefore need redrawing.
    bg_dirty: cell::Cell<bool>,
}

enum Placement {
    Floating(Position),
    Fullscreen,
}

/// Identifies a layer in a `StackView`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerPosition {
    /// Starts from the back (bottom) of the stack.
    FromBack(usize),
    /// Starts from the front (top) of the stack.
    FromFront(usize),
}

impl Placement {
    pub fn compute_offset<S, A, P>(
        &self, size: S, available: A, parent: P,
    ) -> Vec2
    where
        S: Into<Vec2>,
        A: Into<Vec2>,
        P: Into<Vec2>,
    {
        match *self {
            Placement::Floating(ref position) => {
                position.compute_offset(size, available, parent)
            }
            Placement::Fullscreen => Vec2::zero(),
        }
    }
}

// A child view can be wrapped in multiple ways.
enum ChildWrapper<T: View> {
    // Some views include a shadow around.
    Shadow(ShadowView<Layer<T>>),
    // Some include a background.
    Backfilled(Layer<T>),
    // Some views don't (fullscreen views mostly)
    Plain(T),
}

impl<T: View> ChildWrapper<T> {
    fn unwrap(self) -> T {
        match self {
            // ShadowView::into_inner and Layer::into_inner can never fail.
            ChildWrapper::Shadow(shadow) => {
                shadow.into_inner().ok().unwrap().into_inner().ok().unwrap()
            }
            // Layer::into_inner can never fail.
            ChildWrapper::Backfilled(background) => {
                background.into_inner().ok().unwrap()
            }
            ChildWrapper::Plain(layer) => layer,
        }
    }
}

impl<T: View> ChildWrapper<T> {
    /// Returns a reference to the inner view
    pub fn get_inner(&self) -> &View {
        match *self {
            ChildWrapper::Shadow(ref shadow) => shadow.get_inner().get_inner(),
            ChildWrapper::Backfilled(ref background) => background.get_inner(),
            ChildWrapper::Plain(ref layer) => layer,
        }
    }

    /// Returns a mutable reference to the inner view
    pub fn get_inner_mut(&mut self) -> &mut View {
        match *self {
            ChildWrapper::Shadow(ref mut shadow) => {
                shadow.get_inner_mut().get_inner_mut()
            }
            ChildWrapper::Backfilled(ref mut background) => {
                background.get_inner_mut()
            }
            ChildWrapper::Plain(ref mut layer) => layer,
        }
    }
}

// TODO: use macros to make this less ugly?
impl<T: View> View for ChildWrapper<T> {
    fn draw(&self, printer: &Printer) {
        match *self {
            ChildWrapper::Shadow(ref v) => v.draw(printer),
            ChildWrapper::Backfilled(ref v) => v.draw(printer),
            ChildWrapper::Plain(ref v) => v.draw(printer),
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match *self {
            ChildWrapper::Shadow(ref mut v) => v.on_event(event),
            ChildWrapper::Backfilled(ref mut v) => v.on_event(event),
            ChildWrapper::Plain(ref mut v) => v.on_event(event),
        }
    }

    fn layout(&mut self, size: Vec2) {
        match *self {
            ChildWrapper::Shadow(ref mut v) => v.layout(size),
            ChildWrapper::Backfilled(ref mut v) => v.layout(size),
            ChildWrapper::Plain(ref mut v) => v.layout(size),
        }
    }

    fn required_size(&mut self, size: Vec2) -> Vec2 {
        match *self {
            ChildWrapper::Shadow(ref mut v) => v.required_size(size),
            ChildWrapper::Backfilled(ref mut v) => v.required_size(size),
            ChildWrapper::Plain(ref mut v) => v.required_size(size),
        }
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        match *self {
            ChildWrapper::Shadow(ref mut v) => v.take_focus(source),
            ChildWrapper::Backfilled(ref mut v) => v.take_focus(source),
            ChildWrapper::Plain(ref mut v) => v.take_focus(source),
        }
    }

    fn call_on_any<'a>(
        &mut self, selector: &Selector, callback: Box<FnMut(&mut Any) + 'a>,
    ) {
        match *self {
            ChildWrapper::Shadow(ref mut v) => {
                v.call_on_any(selector, callback)
            }
            ChildWrapper::Backfilled(ref mut v) => {
                v.call_on_any(selector, callback)
            }
            ChildWrapper::Plain(ref mut v) => {
                v.call_on_any(selector, callback)
            }
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<(), ()> {
        match *self {
            ChildWrapper::Shadow(ref mut v) => v.focus_view(selector),
            ChildWrapper::Backfilled(ref mut v) => v.focus_view(selector),
            ChildWrapper::Plain(ref mut v) => v.focus_view(selector),
        }
    }
}

struct Child {
    view: ChildWrapper<ViewBox>,
    size: Vec2,
    placement: Placement,

    // We cannot call `take_focus` until we've called `layout()`
    // (for instance, a textView must know it will scroll to be focusable).
    // So we want to call `take_focus` right after the first call to `layout`.
    // This flag remembers when we've done that.
    virgin: bool,
}

new_default!(StackView);

impl StackView {
    /// Creates a new empty StackView
    pub fn new() -> Self {
        StackView {
            layers: Vec::new(),
            last_size: Vec2::zero(),
            bg_dirty: cell::Cell::new(true),
        }
    }

    /// Adds a new full-screen layer on top of the stack.
    ///
    /// Fullscreen layers have no shadow.
    pub fn add_fullscreen_layer<T>(&mut self, view: T)
    where
        T: IntoBoxedView,
    {
        let boxed = ViewBox::boxed(view);
        self.layers.push(Child {
            view: ChildWrapper::Backfilled(Layer::new(boxed)),
            size: Vec2::zero(),
            placement: Placement::Fullscreen,
            virgin: true,
        });
    }

    /// Adds new view on top of the stack in the center of the screen.
    pub fn add_layer<T>(&mut self, view: T)
    where
        T: IntoBoxedView,
    {
        self.add_layer_at(Position::center(), view);
    }

    /// Adds new view on top of the stack in the center of the screen.
    ///
    /// Chainable variant.
    pub fn layer<T>(self, view: T) -> Self
    where
        T: IntoBoxedView,
    {
        self.with(|s| s.add_layer(view))
    }

    /// Returns a reference to the layer at the given position.
    pub fn get(&self, pos: LayerPosition) -> Option<&View> {
        let i = self.get_index(pos);
        self.layers.get(i).map(|child| child.view.get_inner())
    }

    /// Returns a mutable reference to the layer at the given position.
    pub fn get_mut(&mut self, pos: LayerPosition) -> Option<&mut View> {
        let i = self.get_index(pos);
        self.layers
            .get_mut(i)
            .map(|child| child.view.get_inner_mut())
    }

    /// Looks for the layer containing a view with the given ID.
    ///
    /// Returns `Some(pos)` if `self.get(pos)` has the given ID,
    /// or is a parent of a view with this ID.
    ///
    /// Returns `None` if the given ID is not found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::views::{TextView, StackView, Dialog, LayerPosition};
    /// # use cursive::view::Identifiable;
    /// # fn main() {
    /// let mut stack = StackView::new();
    /// stack.add_layer(TextView::new("Back"));
    /// stack.add_layer(Dialog::around(TextView::new("Middle").with_id("text")));
    /// stack.add_layer(TextView::new("Front"));
    ///
    /// assert_eq!(stack.find_layer_from_id("text"), Some(LayerPosition::FromBack(1)));
    /// # }
    /// ```
    pub fn find_layer_from_id(&mut self, id: &str) -> Option<LayerPosition> {
        let selector = Selector::Id(id);

        for (i, child) in self.layers.iter_mut().enumerate() {
            let mut found = false;
            child
                .view
                .call_on_any(&selector, Box::new(|_| found = true));
            if found {
                return Some(LayerPosition::FromBack(i));
            }
        }

        None
    }

    /// Adds a new full-screen layer on top of the stack.
    ///
    /// Chainable variant.
    pub fn fullscreen_layer<T>(self, view: T) -> Self
    where
        T: IntoBoxedView,
    {
        self.with(|s| s.add_fullscreen_layer(view))
    }

    /// Adds a new transparent layer on top of the stack.
    ///
    /// Chainable variant.
    pub fn transparent_layer<T>(self, view: T) -> Self
    where
        T: IntoBoxedView,
    {
        self.with(|s| s.add_transparent_layer(view))
    }

    /// Adds a view on top of the stack.
    pub fn add_layer_at<T>(&mut self, position: Position, view: T)
    where
        T: IntoBoxedView,
    {
        let boxed = ViewBox::boxed(view);
        self.layers.push(Child {
            // Skip padding for absolute/parent-placed views
            view: ChildWrapper::Shadow(
                ShadowView::new(Layer::new(boxed))
                    .top_padding(position.y == Offset::Center)
                    .left_padding(position.x == Offset::Center),
            ),
            size: Vec2::new(0, 0),
            placement: Placement::Floating(position),
            virgin: true,
        });
    }

    /// Adds a transparent view on top of the stack in the center of the screen.
    pub fn add_transparent_layer<T>(&mut self, view: T)
    where
        T: IntoBoxedView,
    {
        self.add_transparent_layer_at(Position::center(), view);
    }

    /// Adds a transparent view on top of the stack.
    pub fn add_transparent_layer_at<T>(&mut self, position: Position, view: T)
    where
        T: IntoBoxedView,
    {
        let boxed = ViewBox::boxed(view);
        self.layers.push(Child {
            view: ChildWrapper::Plain(boxed),
            size: Vec2::new(0, 0),
            placement: Placement::Floating(position),
            virgin: true,
        });
    }

    /// Adds a view on top of the stack.
    ///
    /// Chainable variant.
    pub fn layer_at<T>(self, position: Position, view: T) -> Self
    where
        T: IntoBoxedView,
    {
        self.with(|s| s.add_layer_at(position, view))
    }

    /// Remove the top-most layer.
    pub fn pop_layer(&mut self) -> Option<Box<View>> {
        self.bg_dirty.set(true);
        self.layers
            .pop()
            .map(|child| child.view)
            .map(ChildWrapper::unwrap)
            .map(ViewBox::unwrap)
    }

    /// Computes the offset of the current top view.
    pub fn offset(&self) -> Vec2 {
        let mut previous = Vec2::zero();
        for layer in &self.layers {
            let offset = layer.placement.compute_offset(
                layer.size,
                self.last_size,
                previous,
            );
            previous = offset;
        }
        previous
    }

    /// Returns the size for each layer in this view.
    pub fn layer_sizes(&self) -> Vec<Vec2> {
        self.layers.iter().map(|layer| layer.size).collect()
    }

    fn get_index(&self, pos: LayerPosition) -> usize {
        match pos {
            LayerPosition::FromBack(i) => i,
            LayerPosition::FromFront(i) => self.layers.len() - i - 1,
        }
    }

    /// Moves a layer to a new position in the stack.
    ///
    /// This only affects the elevation of a layer (whether it is drawn over
    /// or under other views).
    pub fn move_layer(&mut self, from: LayerPosition, to: LayerPosition) {
        // Convert relative positions to indices in the array
        let from_i = self.get_index(from);
        let to_i = self.get_index(to);

        let removed = self.layers.remove(from_i);

        self.layers.insert(to_i, removed);
    }

    /// Brings the given view to the front of the stack.
    pub fn move_to_front(&mut self, layer: LayerPosition) {
        self.move_layer(layer, LayerPosition::FromFront(0));
    }

    /// Pushes the given view to the back of the stack.
    pub fn move_to_back(&mut self, layer: LayerPosition) {
        self.move_layer(layer, LayerPosition::FromBack(0));
    }

    /// Moves a layer to a new position on the screen.
    ///
    /// Has no effect on fullscreen layers
    /// Has no effect if layer is not found
    pub fn reposition_layer(
        &mut self, layer: LayerPosition, position: Position,
    ) {
        let i = self.get_index(layer);
        let child = match self.layers.get_mut(i) {
            Some(i) => i,
            None => return,
        };
        match child.placement {
            Placement::Floating(_) => {
                child.placement = Placement::Floating(position);
                self.bg_dirty.set(true);
            }
            Placement::Fullscreen => (),
        }
    }

    /// Background drawing
    ///
    /// Drawing functions are split into forground and background to
    /// ease inserting layers under the stackview but above it's background
    /// you probably just want to call draw()
    pub fn draw_bg(&self, printer: &Printer) {
        // If the background is dirty draw a new background
        if self.bg_dirty.get() {
            for y in 0..printer.size.y {
                printer.with_color(ColorStyle::background(), |printer| {
                    printer.print_hline((0, y), printer.size.x, " ");
                });
            }

            // set background as clean, so we don't need to do this every frame
            self.bg_dirty.set(false);
        }
    }

    /// Forground drawing
    ///
    /// Drawing functions are split into forground and background to
    /// ease inserting layers under the stackview but above it's background
    /// you probably just want to call draw()
    pub fn draw_fg(&self, printer: &Printer) {
        let last = self.layers.len();
        printer.with_color(ColorStyle::primary(), |printer| {
            for (i, (v, offset)) in
                StackPositionIterator::new(self.layers.iter(), printer.size)
                    .enumerate()
            {
                v.view.draw(&printer.sub_printer(
                    offset,
                    v.size,
                    i + 1 == last,
                ));
            }
        });
    }
}

struct StackPositionIterator<R: Deref<Target = Child>, I: Iterator<Item = R>> {
    inner: I,
    previous: Vec2,
    total_size: Vec2,
}

impl<R: Deref<Target = Child>, I: Iterator<Item = R>>
    StackPositionIterator<R, I>
{
    /// Returns a new StackPositionIterator
    pub fn new(inner: I, total_size: Vec2) -> Self {
        let previous = Vec2::zero();
        StackPositionIterator {
            inner,
            previous,
            total_size,
        }
    }
}

impl<R: Deref<Target = Child>, I: Iterator<Item = R>> Iterator
    for StackPositionIterator<R, I>
{
    type Item = (R, Vec2);

    fn next(&mut self) -> Option<(R, Vec2)> {
        self.inner.next().map(|v| {
            let offset = v.placement.compute_offset(
                v.size,
                self.total_size,
                self.previous,
            );

            self.previous = offset;

            // eprintln!("{:?}", offset);
            (v, offset)
        })
    }
}

impl View for StackView {
    fn draw(&self, printer: &Printer) {
        // This function is included for compat with the view trait,
        // it should behave the same as calling them seperately, but does
        // not pause to let you insert in between the layers.
        self.draw_bg(printer);
        self.draw_fg(printer);
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if event == Event::WindowResize {
            self.bg_dirty.set(true);
        }
        // Use the stack position iterator to get the offset of the top layer.
        // TODO: save it instead when drawing?
        match StackPositionIterator::new(
            self.layers.iter_mut(),
            self.last_size,
        ).last()
        {
            None => EventResult::Ignored,
            Some((v, offset)) => v.view.on_event(event.relativized(offset)),
        }
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;

        // The call has been made, we can't ask for more space anymore.
        // Let's make do with what we have.

        for layer in &mut self.layers {
            // Give each guy what he asks for, within the budget constraints.
            let size = Vec2::min(size, layer.view.required_size(size));
            layer.size = size;
            layer.view.layout(layer.size);

            // We need to call `layout()` on the view before giving it focus
            // for the first time. Otherwise it will not be properly set up.
            // Ex: examples/lorem.rs: the text view takes focus because it's
            // scrolling, but it only knows that after a call to `layout()`.
            if layer.virgin {
                layer.view.take_focus(Direction::none());
                layer.virgin = false;
            }
        }
    }

    fn required_size(&mut self, size: Vec2) -> Vec2 {
        // The min size is the max of all children's

        self.layers
            .iter_mut()
            .map(|layer| layer.view.required_size(size))
            .fold(Vec2::new(1, 1), Vec2::max)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        match self.layers.last_mut() {
            None => false,
            Some(v) => v.view.take_focus(source),
        }
    }

    fn call_on_any<'a>(
        &mut self, selector: &Selector,
        mut callback: Box<FnMut(&mut Any) + 'a>,
    ) {
        for layer in &mut self.layers {
            layer
                .view
                .call_on_any(selector, Box::new(|any| callback(any)));
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<(), ()> {
        for layer in &mut self.layers {
            if layer.view.focus_view(selector).is_ok() {
                return Ok(());
            }
        }

        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use views::TextView;

    #[test]
    fn pop_add() {
        // Start with a simple stack
        let mut stack = StackView::new().layer(TextView::new("1"));

        // And keep poping and re-pushing the view
        for _ in 0..20 {
            let layer = stack.pop_layer().unwrap();
            stack.add_layer(layer);
        }

        // We want to make sure we don't add any layer of Box'ing
        let layer = stack.pop_layer().unwrap();
        let text: Box<TextView> = layer.as_boxed_any().downcast().unwrap();

        assert_eq!(text.get_content().source(), "1");
    }

    #[test]
    fn move_layer_works() {
        let mut stack = StackView::new()
            .layer(TextView::new("1"))
            .layer(TextView::new("2"))
            .layer(TextView::new("3"))
            .layer(TextView::new("4"));

        // Try moving views around, make sure we have the expected result

        // 1,2,3,4
        stack.move_layer(
            LayerPosition::FromFront(0),
            LayerPosition::FromBack(0),
        );

        // 4,1,2,3
        stack.move_layer(
            LayerPosition::FromBack(0),
            LayerPosition::FromFront(0),
        );
        // 1,2,3,4
        stack.move_layer(
            LayerPosition::FromFront(1),
            LayerPosition::FromFront(0),
        );
        // 1,2,4,3

        let layer = stack.pop_layer().unwrap();
        let text: Box<TextView> = layer.as_boxed_any().downcast().unwrap();
        assert_eq!(text.get_content().source(), "3");

        let layer = stack.pop_layer().unwrap();
        let text: Box<TextView> = layer.as_boxed_any().downcast().unwrap();
        assert_eq!(text.get_content().source(), "4");

        let layer = stack.pop_layer().unwrap();
        let text: Box<TextView> = layer.as_boxed_any().downcast().unwrap();
        assert_eq!(text.get_content().source(), "2");

        let layer = stack.pop_layer().unwrap();
        let text: Box<TextView> = layer.as_boxed_any().downcast().unwrap();
        assert_eq!(text.get_content().source(), "1");

        assert!(stack.pop_layer().is_none());
    }

    #[test]
    fn get() {
        let mut stack = StackView::new()
            .layer(TextView::new("1"))
            .layer(TextView::new("2"));

        assert!(
            stack
                .get(LayerPosition::FromFront(0))
                .unwrap()
                .as_any()
                .downcast_ref::<ViewBox>()
                .unwrap()
                .with_view(|v| v.as_any().is::<TextView>())
                .unwrap()
        );
        assert!(
            stack
                .get(LayerPosition::FromBack(0))
                .unwrap()
                .as_any()
                .downcast_ref::<ViewBox>()
                .unwrap()
                .with_view(|v| v.as_any().is::<TextView>())
                .unwrap()
        );
        assert!(
            stack
                .get_mut(LayerPosition::FromFront(0))
                .unwrap()
                .as_any_mut()
                .downcast_mut::<ViewBox>()
                .unwrap()
                .with_view_mut(|v| v.as_any_mut().is::<TextView>())
                .unwrap()
        );
        assert!(
            stack
                .get_mut(LayerPosition::FromBack(0))
                .unwrap()
                .as_any_mut()
                .downcast_mut::<ViewBox>()
                .unwrap()
                .with_view_mut(|v| v.as_any_mut().is::<TextView>())
                .unwrap()
        );
    }
}
