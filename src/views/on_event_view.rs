use Cursive;
use With;
use event::{Callback, Event, EventResult};
use std::collections::HashMap;
use std::rc::Rc;
use view::{View, ViewWrapper};

/// A wrapper view that can react to events.
///
/// This view registers a set of callbacks tied to specific events, to be run
/// in certain conditions.
///
/// **Note**: only one callback can be registered per event. Trying to register
/// a new one will replace any existing one for that event.
///
/// * Some callbacks are called only for vents ignored by the wrapped view
///   (those registered by [`on_event`] or [`on_event_inner`])
/// * Others are processed first, and can control whether the child view should
///   be given the event (those registered by [`on_pre_event`] or
///   [`on_pre_event_inner`]).
///
/// "Inner" callbacks ([`on_event_inner`] and [`on_pre_event_inner`]) are given
/// a reference to the inner wrapped view (but not to the `Cursive` root). They
/// can then return another callback, taking only a `&mut Cursive` root as
/// argument.
///
/// "Simple" callbacks ([`on_event`] and [`on_pre_event`]) skip this first
/// phase and are only called with a `&mut Cursive`.
///
/// [`on_event`]: struct.OnEventView.html#method.on_event
/// [`on_pre_event`]: struct.OnEventView.html#method.on_pre_event
/// [`on_event_inner`]: struct.OnEventView.html#method.on_event_inner
/// [`on_pre_event_inner`]: struct.OnEventView.html#method.on_pre_event_inner
///
/// # Examples
///
/// ```
/// # use cursive::event;;
/// # use cursive::views::{OnEventView, TextView};
/// let view = OnEventView::new(TextView::new("This view has an event!"))
///                         .on_event('q', |s| s.quit())
///                         .on_event(event::Key::Esc, |s| s.quit());
/// ```
pub struct OnEventView<T: View> {
    view: T,
    callbacks: HashMap<Event, Action<T>>,
}

type InnerCallback<T> = Rc<Box<Fn(&mut T) -> Option<EventResult>>>;

struct Action<T> {
    phase: TriggerPhase,
    callback: InnerCallback<T>,
}

impl<T> Clone for Action<T> {
    fn clone(&self) -> Self {
        Action {
            phase: self.phase.clone(),
            callback: Rc::clone(&self.callback),
        }
    }
}

#[derive(PartialEq, Clone)]
enum TriggerPhase {
    BeforeChild,
    AfterChild,
}

impl<T: View> OnEventView<T> {
    /// Wraps the given view in a new OnEventView.
    pub fn new(view: T) -> Self {
        OnEventView {
            view: view,
            callbacks: HashMap::new(),
        }
    }

    /// Registers a callback when the given event is ignored by the child.
    ///
    /// Chainable variant.
    pub fn on_event<F, E>(self, event: E, cb: F) -> Self
    where
        E: Into<Event>,
        F: 'static + Fn(&mut Cursive),
    {
        self.with(|s| s.set_on_event(event, cb))
    }

    /// Registers a callback when the given event is received.
    ///
    /// The child will never receive this event.
    ///
    /// Chainable variant.
    pub fn on_pre_event<F, E>(self, event: E, cb: F) -> Self
    where
        E: Into<Event>,
        F: 'static + Fn(&mut Cursive),
    {
        self.with(|s| s.set_on_pre_event(event, cb))
    }

    /// Registers a callback when the given event is received.
    ///
    /// The given callback will be run before the child view sees the event.
    ///
    /// * If the result is `None`, then the child view is given the event as
    ///   usual.
    /// * Otherwise, it bypasses the child view and directly processes the
    ///   result.
    ///
    /// Chainable variant.
    pub fn on_pre_event_inner<F, E>(self, event: E, cb: F) -> Self
    where
        E: Into<Event>,
        F: Fn(&mut T) -> Option<EventResult> + 'static,
    {
        self.with(|s| s.set_on_pre_event_inner(event, cb))
    }

    /// Registers a callback when the given event is ignored by the child.
    ///
    /// If the child view ignores the event, `cb` will be called with the
    /// child view as argument.
    /// If the result is not `None`, it will be processed as well.
    ///
    /// Chainable variant.
    pub fn on_event_inner<F, E>(self, event: E, cb: F) -> Self
    where
        E: Into<Event>,
        F: Fn(&mut T) -> Option<EventResult> + 'static,
    {
        self.with(|s| s.set_on_event_inner(event, cb))
    }

    /// Registers a callback when the given event is ignored by the child.
    pub fn set_on_event<F, E>(&mut self, event: E, cb: F)
    where
        E: Into<Event>,
        F: Fn(&mut Cursive) + 'static,
    {
        let cb = Callback::from_fn(cb);
        let action =
            move |_: &mut T| Some(EventResult::Consumed(Some(cb.clone())));

        self.set_on_event_inner(event, action);
    }

    /// Registers a callback when the given event is received.
    ///
    /// The child will never receive this event.
    pub fn set_on_pre_event<F, E>(&mut self, event: E, cb: F)
    where
        E: Into<Event>,
        F: 'static + Fn(&mut Cursive),
    {
        let cb = Callback::from_fn(cb);
        // We want to clone the Callback every time we call the closure
        let action =
            move |_: &mut T| Some(EventResult::Consumed(Some(cb.clone())));

        self.set_on_pre_event_inner(event, action);
    }

    /// Registers a callback when the given event is received.
    ///
    /// The given callback will be run before the child view sees the event.
    ///
    /// * If the result is `None`, then the child view is given the event as
    ///   usual.
    /// * Otherwise, it bypasses the child view and directly processes the
    ///   result.
    pub fn set_on_pre_event_inner<F, E>(&mut self, event: E, cb: F)
    where
        E: Into<Event>,
        F: Fn(&mut T) -> Option<EventResult> + 'static,
    {
        self.callbacks.insert(
            event.into(),
            Action {
                phase: TriggerPhase::BeforeChild,
                callback: Rc::new(Box::new(cb)),
            },
        );
    }

    /// Registers a callback when the given event is ignored by the child.
    ///
    /// If the child view ignores the event, `cb` will be called with the
    /// child view as argument.
    /// If the result is not `None`, it will be processed as well.
    pub fn set_on_event_inner<F, E>(&mut self, event: E, cb: F)
    where
        E: Into<Event>,
        F: Fn(&mut T) -> Option<EventResult> + 'static,
    {
        self.callbacks.insert(
            event.into(),
            Action {
                phase: TriggerPhase::AfterChild,
                callback: Rc::new(Box::new(cb)),
            },
        );
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for OnEventView<T> {
    wrap_impl!(self.view: T);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        let action = self.callbacks.get(&event).cloned();
        let pre_child = action
            .as_ref()
            .map(|a| a.phase == TriggerPhase::BeforeChild)
            .unwrap_or(false);

        if pre_child {
            action
                .and_then(|a| (*a.callback)(&mut self.view))
                .unwrap_or_else(|| self.view.on_event(event))
        } else {
            self.view.on_event(event).or_else(|| {
                action
                    .and_then(|a| (*a.callback)(&mut self.view))
                    .unwrap_or(EventResult::Ignored)
            })
        }
    }
}
