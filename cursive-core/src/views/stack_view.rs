//! Helpers for `StackView`.
//!
//! This defines `LayerConfig` and a couple of struct that implement `Into<LayerConfig>`.
//!
//! To be used with `StackView::add_layer`.
use crate::{
    direction::Direction,
    event::{AnyCb, Event, EventResult},
    style::PaletteStyle,
    view::{
        CannotFocus, IntoBoxedView, Offset, Position, Selector, View, ViewNotFound, ViewWrapper,
    },
    views::{BoxedView, CircularFocus, Layer, ShadowView},
    Printer, Vec2, With,
};
use std::ops::Deref;

/// Simple stack of views.
/// Only the top-most view is active and can receive input.
pub struct StackView {
    // Store layers from back to front.
    layers: Vec<Child>,
    last_size: Vec2,
    // Flag indicates if undrawn areas of the background are exposed
    // and therefore need redrawing.
    // TODO: this is broken! Transparent views could change their content and lead to weirdness.
    // Instead, just rely on buffered backend.
    bg_dirty: std::sync::atomic::AtomicBool,
}

// This is a poor man's optional parameter, or kinda builder pattern.
// We can give either `view`, or `Modeless(view)`, or `LayerAt(position, NoShadow(view))`.

/// Configuration for a layer in a `StackView`.
pub struct LayerConfig<V> {
    view: V,
    modal: bool,
    placement: Placement,
    wrapper: WrapperType,
}

/// Make the layer non-modal.
///
/// If this layer ignores events, it will go to the layer behind it.
pub struct Modeless<T>(pub T);

/// Make a layer full-screen.
///
/// You probably _also_ want to make the layer `NoShadow`, or even `Transparent`.
pub struct Fullscreen<T>(pub T);

/// Make the layer not include a shadow.
pub struct NoShadow<T>(pub T);

/// Make the layer transparent.
///
/// No background will be filled.
///
/// This implicitly also removes shadows.
pub struct Transparent<T>(pub T);

/// Place the layer at the given position.
pub struct LayerAt<T>(pub Position, pub T);

impl<T, V> From<Transparent<T>> for LayerConfig<V>
where
    T: Into<LayerConfig<V>>,
{
    fn from(other: Transparent<T>) -> Self {
        other.0.into().with(|config| {
            config.wrapper = WrapperType::Plain;
        })
    }
}

impl<T, V> From<NoShadow<T>> for LayerConfig<V>
where
    T: Into<LayerConfig<V>>,
{
    fn from(other: NoShadow<T>) -> Self {
        other.0.into().with(|config| {
            if let WrapperType::Shadow = config.wrapper {
                // We don't want to undo a transparent specifier.
                config.wrapper = WrapperType::Backfilled;
            }
        })
    }
}

impl<T, V> From<Fullscreen<T>> for LayerConfig<V>
where
    T: Into<LayerConfig<V>>,
{
    fn from(other: Fullscreen<T>) -> Self {
        other.0.into().with(|config| {
            config.placement = Placement::Fullscreen;
        })
    }
}

impl<T, V> From<LayerAt<T>> for LayerConfig<V>
where
    T: Into<LayerConfig<V>>,
{
    fn from(other: LayerAt<T>) -> Self {
        other.1.into().with(|config| {
            config.placement = Placement::Floating(other.0);
        })
    }
}

impl<T, V> From<Modeless<T>> for LayerConfig<V>
where
    T: Into<LayerConfig<V>>,
{
    fn from(other: Modeless<T>) -> Self {
        other.0.into().with(|config| config.modal = false)
    }
}

// We need the IntoBoxedView trait to guarantee we don't overlap with the other impl
impl<V: IntoBoxedView> From<V> for LayerConfig<V> {
    fn from(view: V) -> Self {
        LayerConfig {
            view,
            modal: true,
            placement: Placement::Floating(Position::center()),
            wrapper: WrapperType::Shadow,
        }
    }
}

/// Where should the view be on the screen (per dimension).
enum Placement {
    /// View is floating at a specific position.
    Floating(Position),

    /// View is full-screen; it should not have a 1-cell border.
    Fullscreen,
}

/// Identifies a layer in a `StackView`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerPosition {
    /// Starts from the back (bottom) of the stack.
    FromBack(usize),
    /// Starts from the front (top) of the stack.
    FromFront(usize),
}

impl Placement {
    pub fn compute_offset<S, A, P>(&self, size: S, available: A, parent: P) -> Vec2
    where
        S: Into<Vec2>,
        A: Into<Vec2>,
        P: Into<Vec2>,
    {
        match *self {
            Placement::Floating(ref position) => position.compute_offset(size, available, parent),
            Placement::Fullscreen => Vec2::zero(),
        }
    }
}

/// A child view can be wrapped in multiple ways.
enum ChildWrapper<T: View> {
    // Some views include a shadow around.
    Shadow(ShadowView<Layer<T>>),

    // Some include only include a background.
    Backfilled(Layer<T>),

    // Some views don't even have a background (they'll be transparent).
    Plain(T),
}

enum WrapperType {
    Shadow,
    Backfilled,
    Plain,
}

impl WrapperType {
    pub fn wrap<V: ViewWrapper>(self, view: V, padding: crate::XY<bool>) -> ChildWrapper<V> {
        match self {
            Self::Shadow => ChildWrapper::Shadow(
                ShadowView::new(Layer::new(view))
                    .top_padding(padding.y)
                    .left_padding(padding.x),
            ),
            Self::Backfilled => ChildWrapper::Backfilled(Layer::new(view)),
            Self::Plain => ChildWrapper::Plain(view),
        }
    }
}

impl<T: View> ChildWrapper<T> {
    fn into_inner(self) -> T {
        match self {
            // All these into_inner() can never fail.
            // (ShadowView, Layer, CircularFocus)
            ChildWrapper::Shadow(shadow) => {
                shadow.into_inner().ok().unwrap().into_inner().ok().unwrap()
            }
            // Layer::into_inner can never fail.
            ChildWrapper::Backfilled(background) => background.into_inner().ok().unwrap(),
            ChildWrapper::Plain(layer) => layer,
        }
    }
}

impl<T: View> ChildWrapper<T> {
    /// Returns a reference to the inner view
    pub fn get_inner(&self) -> &T {
        match *self {
            ChildWrapper::Shadow(ref shadow) => shadow.get_inner().get_inner(),
            ChildWrapper::Backfilled(ref background) => background.get_inner(),
            ChildWrapper::Plain(ref layer) => layer,
        }
    }

    /// Returns a mutable reference to the inner view
    pub fn get_inner_mut(&mut self) -> &mut T {
        match *self {
            ChildWrapper::Shadow(ref mut shadow) => shadow.get_inner_mut().get_inner_mut(),
            ChildWrapper::Backfilled(ref mut background) => background.get_inner_mut(),
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

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        match *self {
            ChildWrapper::Shadow(ref mut v) => v.take_focus(source),
            ChildWrapper::Backfilled(ref mut v) => v.take_focus(source),
            ChildWrapper::Plain(ref mut v) => v.take_focus(source),
        }
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        match *self {
            ChildWrapper::Shadow(ref mut v) => v.call_on_any(selector, callback),
            ChildWrapper::Backfilled(ref mut v) => v.call_on_any(selector, callback),
            ChildWrapper::Plain(ref mut v) => v.call_on_any(selector, callback),
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        match *self {
            ChildWrapper::Shadow(ref mut v) => v.focus_view(selector),
            ChildWrapper::Backfilled(ref mut v) => v.focus_view(selector),
            ChildWrapper::Plain(ref mut v) => v.focus_view(selector),
        }
    }
}

struct Child {
    view: CircularFocus<ChildWrapper<BoxedView>>,
    size: Vec2,
    placement: Placement,
    modal: bool,

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
            bg_dirty: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Returns the number of layers in this `StackView`.
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Returns `true` if there is no layer in this `StackView`.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Returns `true` if `position` points to a valid layer.
    ///
    /// Returns `false` if it exceeds the bounds.
    pub fn fits(&self, position: LayerPosition) -> bool {
        let i = match position {
            LayerPosition::FromBack(i) | LayerPosition::FromFront(i) => i,
        };

        i < self.len()
    }

    /// Adds a new full-screen layer on top of the stack.
    ///
    /// Fullscreen layers have no shadow.
    ///
    /// Soon deprecated in favor of `add_layer(Fullscreen(view))`.
    pub fn add_fullscreen_layer<T>(&mut self, view: T)
    where
        T: IntoBoxedView,
    {
        self.add_layer(NoShadow(Fullscreen(view)));
    }

    /// Adds new view on top of the stack in the center of the screen.
    pub fn add_layer<T, V>(&mut self, view: T)
    where
        T: Into<LayerConfig<V>>,
        V: IntoBoxedView,
    {
        let LayerConfig {
            view,
            modal,
            placement,
            wrapper,
        } = view.into();

        let position = match placement {
            Placement::Floating(position) => position,
            _ => Position::center(),
        };

        let view = BoxedView::boxed(view.into_boxed_view());
        let view = wrapper.wrap(view, position.map(|x| x == Offset::Center));
        let view = CircularFocus::new(view).wrap_tab();

        self.layers.push(Child {
            view,
            modal,
            placement,
            size: Vec2::zero(),
            virgin: true,
        });
    }

    /// Adds new view on top of the stack in the center of the screen.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn layer<T, V>(self, view: T) -> Self
    where
        T: Into<LayerConfig<V>>,
        V: IntoBoxedView,
    {
        self.with(|s| s.add_layer(view))
    }

    /// Returns a reference to the layer at the given position.
    pub fn get(&self, pos: LayerPosition) -> Option<&dyn View> {
        self.get_index(pos).and_then(|i| {
            self.layers
                .get(i)
                .map(|child| &**child.view.get_inner().get_inner())
        })
    }

    /// Returns a mutable reference to the layer at the given position.
    pub fn get_mut(&mut self, pos: LayerPosition) -> Option<&mut dyn View> {
        self.get_index(pos).and_then(move |i| {
            self.layers
                .get_mut(i)
                .map(|child| &mut **child.view.get_inner_mut().get_inner_mut())
        })
    }

    /// Looks for the layer containing a view with the given name.
    ///
    /// Returns `Some(pos)` if `self.get(pos)` has the given name,
    /// or is a parent of a view with this name.
    ///
    /// Returns `None` if the given name is not found.
    ///
    /// Note that the returned position may be invalidated if some layers are
    /// removed from the view.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::views::{TextView, StackView, Dialog, LayerPosition};
    /// # use cursive_core::view::Nameable;
    /// let mut stack = StackView::new();
    /// stack.add_layer(TextView::new("Back"));
    /// stack.add_layer(Dialog::around(TextView::new("Middle").with_name("text")));
    /// stack.add_layer(TextView::new("Front"));
    ///
    /// assert_eq!(
    ///     stack.find_layer_from_name("text"),
    ///     Some(LayerPosition::FromBack(1))
    /// );
    /// ```
    pub fn find_layer_from_name(&mut self, id: &str) -> Option<LayerPosition> {
        let selector = Selector::Name(id);

        for (i, child) in self.layers.iter_mut().enumerate() {
            let mut found = false;
            child.view.call_on_any(&selector, &mut |_| found = true);
            if found {
                return Some(LayerPosition::FromBack(i));
            }
        }

        None
    }

    /// Adds a new full-screen layer on top of the stack.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn fullscreen_layer<T>(self, view: T) -> Self
    where
        T: IntoBoxedView,
    {
        self.with(|s| s.add_fullscreen_layer(view))
    }

    /// Adds a new transparent layer on top of the stack.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn transparent_layer<T>(self, view: T) -> Self
    where
        T: IntoBoxedView,
    {
        self.with(|s| s.add_transparent_layer(view))
    }

    /// Adds a view on top of the stack.
    ///
    /// Soon deprecated in favor of `add_layer(LayerAt(position, view))`.
    pub fn add_layer_at<T>(&mut self, position: Position, view: T)
    where
        T: IntoBoxedView,
    {
        self.add_layer(LayerAt(position, view));
    }

    /// Adds a transparent view on top of the stack in the center of the screen.
    ///
    /// Soon deprecated in favor of `add_layer(Transparent(view))`.
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
        self.add_layer(LayerAt(position, Transparent(view)));
    }

    /// Adds a view on top of the stack at the given position.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn layer_at<T>(self, position: Position, view: T) -> Self
    where
        T: IntoBoxedView,
    {
        self.with(|s| s.add_layer_at(position, view))
    }

    /// Remove a layer from this `StackView`.
    ///
    /// # Panics
    ///
    /// If the given position is out of bounds.
    pub fn remove_layer(&mut self, position: LayerPosition) -> Box<dyn View> {
        self.set_dirty();
        let i = self.get_index(position).unwrap();
        self.layers
            .remove(i)
            .view
            .into_inner()
            .ok()
            .unwrap()
            .into_inner()
            .unwrap()
    }

    fn set_dirty(&self) {
        self.bg_dirty
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn undirty(&self) {
        self.bg_dirty
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    fn is_dirty(&self) -> bool {
        self.bg_dirty.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Remove the top-most layer.
    pub fn pop_layer(&mut self) -> Option<Box<dyn View>> {
        self.set_dirty();
        self.layers
            .pop()
            .map(|child| child.view)
            .map(CircularFocus::into_inner)
            .map(Result::ok)
            .map(Option::unwrap)
            .map(ChildWrapper::into_inner)
            .map(BoxedView::unwrap)
    }

    fn layer_offsets(&self) -> impl Iterator<Item = Vec2> + '_ {
        let last_size = self.last_size;
        let mut previous = Vec2::zero();
        self.layers.iter().map(move |layer| {
            let offset = layer
                .placement
                .compute_offset(layer.size, last_size, previous);
            previous = offset;
            offset
        })
    }

    /// Computes the offset of the selected view.
    pub fn layer_offset(&self, pos: LayerPosition) -> Option<Vec2> {
        let n = self.get_index(pos)?;
        self.layer_offsets().nth(n)
    }

    /// Computes the offset of the current top view.
    #[deprecated(since = "0.17.0", note = "Use StackView::layer_offset instead.")]
    pub fn offset(&self) -> Vec2 {
        self.layer_offsets().last().unwrap_or_else(Vec2::zero)
    }

    /// Returns the size for each layer in this view.
    pub fn layer_sizes(&self) -> Vec<Vec2> {
        self.layers.iter().map(|layer| layer.size).collect()
    }

    fn get_index(&self, pos: LayerPosition) -> Option<usize> {
        match pos {
            LayerPosition::FromBack(i) => Some(i),
            LayerPosition::FromFront(i) => self.layers.len().checked_sub(i + 1),
        }
    }

    /// Moves a layer to a new position in the stack.
    ///
    /// This only affects the elevation of a layer (whether it is drawn over
    /// or under other views).
    ///
    /// # Panics
    ///
    /// If either `from` or `to` is out of bounds.
    pub fn move_layer(&mut self, from: LayerPosition, to: LayerPosition) {
        // Convert relative positions to indices in the array
        let from = self.get_index(from).unwrap();
        let to = self.get_index(to).unwrap();

        let removed = self.layers.remove(from);
        self.layers.insert(to, removed);
    }

    /// Brings the given view to the front of the stack.
    ///
    /// # Panics
    ///
    /// If `layer` is out of bounds.
    pub fn move_to_front(&mut self, layer: LayerPosition) {
        self.move_layer(layer, LayerPosition::FromFront(0));
    }

    /// Pushes the given view to the back of the stack.
    ///
    /// # Panics
    ///
    /// If `layer` is out of bounds.
    pub fn move_to_back(&mut self, layer: LayerPosition) {
        self.move_layer(layer, LayerPosition::FromBack(0));
    }

    /// Moves a layer to a new position on the screen.
    ///
    /// # Panics
    ///
    /// If `layer` is out of bounds.
    pub fn reposition_layer(&mut self, layer: LayerPosition, position: Position) {
        let i = self.get_index(layer).unwrap();
        let child = &mut self.layers[i];
        match child.placement {
            Placement::Floating(_) => {
                child.placement = Placement::Floating(position);
                self.set_dirty();
            }
            Placement::Fullscreen => (),
        }
    }

    /// Make the given layer (non-)modal.
    pub fn set_modal(&mut self, layer: LayerPosition, modal: bool) {
        let i = self.get_index(layer).unwrap();
        let child = &mut self.layers[i];
        child.modal = modal;
    }

    /// Background drawing
    ///
    /// Drawing functions are split into foreground and background to
    /// ease inserting layers under the stackview but above its background.
    ///
    /// You probably just want to call draw()
    pub fn draw_bg(&self, printer: &Printer) {
        // If the background is dirty draw a new background
        if self.is_dirty() {
            for y in 0..printer.size.y {
                printer.with_style(PaletteStyle::Background, |printer| {
                    printer.print_hline((0, y), printer.size.x, " ");
                });
            }

            // set background as clean, so we don't need to do this every frame
            self.undirty();
        }
    }

    /// Foreground drawing
    ///
    /// Drawing functions are split into foreground and background to
    /// ease inserting layers under the stackview but above its background.
    ///
    /// you probably just want to call draw()
    pub fn draw_fg(&self, printer: &Printer) {
        let last = self.layers.len();
        printer.with_style(PaletteStyle::Background, |printer| {
            for (i, (v, offset)) in
                StackPositionIterator::new(self.layers.iter(), printer.size).enumerate()
            {
                v.view.draw(
                    &printer
                        .offset(offset)
                        .cropped(v.size)
                        .focused(i + 1 == last),
                );
            }
        });
    }
}

/// Iterates on the layers and compute the position of each.
struct StackPositionIterator<I> {
    inner: I,
    previous: Vec2,
    total_size: Vec2,
}

impl<I> StackPositionIterator<I>
where
    I: Iterator,
    I::Item: Deref<Target = Child>,
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

impl<I> Iterator for StackPositionIterator<I>
where
    I: Iterator,
    I::Item: Deref<Target = Child>,
{
    type Item = (I::Item, Vec2);

    fn next(&mut self) -> Option<(I::Item, Vec2)> {
        self.inner.next().map(|v| {
            let offset = v
                .placement
                .compute_offset(v.size, self.total_size, self.previous);

            self.previous = offset;

            // eprintln!("{:?}", offset);
            (v, offset)
        })
    }
}

impl View for StackView {
    fn draw(&self, printer: &Printer) {
        // This function is included for compat with the view trait,
        // it should behave the same as calling them separately, but does
        // not pause to let you insert in between the layers.
        self.draw_bg(printer);
        self.draw_fg(printer);
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if event == Event::WindowResize {
            self.set_dirty();
        }

        // Use the stack position iterator to get the offset of the top layer.
        // TODO: save it instead when drawing?
        let stack_positions: Vec<_> =
            StackPositionIterator::new(self.layers.iter_mut(), self.last_size).collect();

        // Start from the end of the stack (the front-most layer).
        for (v, offset) in stack_positions.into_iter().rev() {
            // Pop up the first view that consumes the event.
            if let event @ EventResult::Consumed(_) = v.view.on_event(event.relativized(offset)) {
                return event;
            }

            // Stop at the first modal view.
            if v.modal {
                break;
            }
        }

        EventResult::Ignored
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
            // Ex: `cursive/examples/lorem.rs`
            // The text view takes focus because it's scrolling, but it only
            // knows that after a call to `layout()`.
            if layer.virgin {
                // Here we can't really forward the callback.
                // So just ignore the result. :(
                layer.view.take_focus(Direction::none()).ok();
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

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        match self.layers.last_mut() {
            None => Err(CannotFocus),
            Some(v) => v.view.take_focus(source),
        }
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        for layer in &mut self.layers {
            layer.view.call_on_any(selector, callback);
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        for layer in &mut self.layers {
            if layer.view.focus_view(selector).is_ok() {
                return Ok(EventResult::Consumed(None));
            }
        }

        Err(ViewNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::TextView;

    #[test]
    fn pop_add() {
        // Start with a simple stack
        let mut stack = StackView::new().layer(TextView::new("1"));

        // And keep popping and re-pushing the view
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
        stack.move_layer(LayerPosition::FromFront(0), LayerPosition::FromBack(0));

        // 4,1,2,3
        stack.move_layer(LayerPosition::FromBack(0), LayerPosition::FromFront(0));
        // 1,2,3,4
        stack.move_layer(LayerPosition::FromFront(1), LayerPosition::FromFront(0));
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

        assert!(stack
            .get(LayerPosition::FromFront(0))
            .unwrap()
            .is::<TextView>());
        assert!(stack
            .get(LayerPosition::FromBack(0))
            .unwrap()
            .is::<TextView>());
        assert!(stack
            .get_mut(LayerPosition::FromFront(0))
            .unwrap()
            .is::<TextView>());
        assert!(stack
            .get_mut(LayerPosition::FromBack(0))
            .unwrap()
            .is::<TextView>());
    }
}

crate::manual_blueprint!(StackView, |config, context| {
    let mut stack = StackView::new();

    // TODO: Use `layers`?
    // Start from the top? (So the yaml file looks like the actual stack?)
    // Use `top_to_bottom` field name to make this clear?
    if let Some(children) = context.resolve(&config["children"])? {
        stack.layers = children;
    }

    Ok(stack)
});

#[cfg(feature = "builder")]
impl crate::builder::Resolvable for Placement {
    fn from_config(
        config: &crate::builder::Config,
        context: &crate::builder::Context,
    ) -> Result<Self, crate::builder::Error> {
        match config.as_str() {
            Some("fullscreen" | "Fullscreen" | "FullScreen") => Ok(Self::Fullscreen),
            _ => Ok(Self::Floating(context.resolve(config)?)),
        }
    }
}

#[cfg(feature = "builder")]
impl crate::builder::Resolvable for Child {
    fn from_config(
        config: &crate::builder::Config,
        context: &crate::builder::Context,
    ) -> Result<Self, crate::builder::Error> {
        let view: crate::views::BoxedView = context.resolve(&config["child"])?;

        let modal: Option<bool> = context.resolve(&config["modal"])?;
        let placement = context.resolve(&config["placement"])?;
        let position: Position = context.resolve(&config["position"])?;

        // Right now only plain layer+shadow views are allowed in configs.
        Ok(Child {
            view: CircularFocus::new(ChildWrapper::Shadow(
                ShadowView::new(Layer::new(view))
                    .top_padding(position.y == Offset::Center)
                    .left_padding(position.x == Offset::Center),
            ))
            .wrap_tab(),
            modal: modal.unwrap_or(true),
            size: Vec2::zero(),
            placement,
            virgin: true,
        })
    }
}
