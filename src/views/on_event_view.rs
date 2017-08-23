use Cursive;
use With;
use event::{Callback, Event, EventResult};
use std::collections::HashMap;

use std::rc::Rc;
use view::{View, ViewWrapper};

/// A simple wrapper view that catches some ignored event from its child.
///
/// If the event doesn't have a corresponding callback, it will stay ignored.
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
    inner: T,
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
            callback: self.callback.clone(),
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
            inner: view,
            callbacks: HashMap::new(),
        }
    }

    /// Registers a callback when the given event is ignored by the child.
    ///
    /// Chainable variant.
    pub fn on_event<F, E>(self, event: E, cb: F) -> Self
    where
        E: Into<Event>,
        F: Fn(&mut Cursive) + 'static,
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
        F: Fn(&mut Cursive) + 'static,
    {
        self.with(|s| s.set_on_pre_event(event, cb))
    }

    /// Registers a callback when the given event is received.
    ///
    /// The given callback will be run before the child view sees the event.
    /// If the result is `None`, then the child view is given the event as usual.
    /// Otherwise, it bypasses the child view and directly processes the result.
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
        F: Fn(&mut Cursive) + 'static,
    {
        let cb = Callback::from_fn(cb);
        let action =
            move |_: &mut T| Some(EventResult::Consumed(Some(cb.clone())));

        self.set_on_pre_event_inner(event, action);
    }

    /// Registers a callback when the given event is received.
    ///
    /// The given callback will be run before the child view sees the event.
    /// If the result is `None`, then the child view is given the event as usual.
    /// Otherwise, it bypasses the child view and directly processes the result.
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
}

impl<T: View> ViewWrapper for OnEventView<T> {
    wrap_impl!(self.inner: T);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        let action = self.callbacks.get(&event).cloned();
        let pre_child = action
            .as_ref()
            .map(|a| a.phase == TriggerPhase::BeforeChild)
            .unwrap_or(false);

        if pre_child {
            action
                .and_then(|a| (*a.callback)(&mut self.inner))
                .unwrap_or_else(|| self.inner.on_event(event))
        } else {
            self.inner.on_event(event).or_else(|| {
                action
                    .and_then(|a| (*a.callback)(&mut self.inner))
                    .unwrap_or(EventResult::Ignored)
            })
        }
    }
}
