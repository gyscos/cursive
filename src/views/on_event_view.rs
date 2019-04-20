use crate::event::{Callback, Event, EventResult, EventTrigger};
use crate::view::{View, ViewWrapper};
use crate::Cursive;
use crate::With;
use std::rc::Rc;

/// A wrapper view that can react to events.
///
/// This view registers a set of callbacks tied to specific events, to be run
/// in certain conditions.
///
/// * Some callbacks are called only for events ignored by the wrapped view.
///
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
    callbacks: Vec<(EventTrigger, Action<T>)>,
}

type InnerCallback<T> = Rc<Box<dyn Fn(&mut T, &Event) -> Option<EventResult>>>;

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
            view,
            callbacks: Vec::new(),
        }
    }

    /// Registers a callback when the given event is ignored by the child.
    ///
    /// Chainable variant.
    ///
    /// # Examples
    ///
    ///
    /// ```rust
    /// # use cursive::views::{OnEventView, DummyView};
    /// # use cursive::event::{Key, EventTrigger};
    /// let view = OnEventView::new(DummyView)
    ///     .on_event('q', |s| s.quit())
    ///     .on_event(Key::Esc, |s| {
    ///         s.pop_layer();
    ///     })
    ///     .on_event(EventTrigger::mouse(), |s| {
    ///         s.add_layer(DummyView);
    ///     });
    /// ```
    pub fn on_event<F, E>(self, trigger: E, cb: F) -> Self
    where
        E: Into<EventTrigger>,
        F: 'static + Fn(&mut Cursive),
    {
        self.with(|s| s.set_on_event(trigger, cb))
    }

    /// Registers a callback when the given event is received.
    ///
    /// The child will never receive this event.
    ///
    /// Chainable variant.
    pub fn on_pre_event<F, E>(self, trigger: E, cb: F) -> Self
    where
        E: Into<EventTrigger>,
        F: 'static + Fn(&mut Cursive),
    {
        self.with(|s| s.set_on_pre_event(trigger, cb))
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
    pub fn on_pre_event_inner<F, E>(self, trigger: E, cb: F) -> Self
    where
        E: Into<EventTrigger>,
        F: Fn(&mut T, &Event) -> Option<EventResult> + 'static,
    {
        self.with(|s| s.set_on_pre_event_inner(trigger, cb))
    }

    /// Registers a callback when the given event is ignored by the child.
    ///
    /// This is an advanced method to get more control.
    /// [`on_event`] may be easier to use.
    ///
    /// If the child view ignores the event, `cb` will be called with the
    /// child view as argument.
    /// If the result is not `None`, it will be processed as well.
    ///
    /// Chainable variant.
    ///
    /// [`on_event`]: OnEventView::on_event()
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::views::{DummyView, OnEventView};
    /// # use cursive::event::{Event, EventTrigger, MouseEvent, EventResult};
    /// let view = OnEventView::new(DummyView)
    ///     .on_event_inner(
    ///         EventTrigger::mouse(),
    ///         |d: &mut DummyView, e: &Event| {
    ///             if let &Event::Mouse { event: MouseEvent::Press(_), .. } = e {
    ///                 // Do something on mouse press
    ///                 Some(EventResult::with_cb(|s| {
    ///                     s.pop_layer();
    ///                 }))
    ///             } else {
    ///                 // Otherwise, don't do anything
    ///                 None
    ///             }
    ///         }
    ///     );
    /// ```
    pub fn on_event_inner<F, E>(self, trigger: E, cb: F) -> Self
    where
        E: Into<EventTrigger>,
        F: Fn(&mut T, &Event) -> Option<EventResult> + 'static,
    {
        self.with(|s| s.set_on_event_inner(trigger, cb))
    }

    /// Registers a callback when the given event is ignored by the child.
    pub fn set_on_event<F, E>(&mut self, trigger: E, cb: F)
    where
        E: Into<EventTrigger>,
        F: Fn(&mut Cursive) + 'static,
    {
        let cb = Callback::from_fn(cb);
        let action = move |_: &mut T, _: &Event| {
            Some(EventResult::Consumed(Some(cb.clone())))
        };

        self.set_on_event_inner(trigger, action);
    }

    /// Registers a callback when the given event is received.
    ///
    /// The child will never receive this event.
    pub fn set_on_pre_event<F, E>(&mut self, trigger: E, cb: F)
    where
        E: Into<EventTrigger>,
        F: 'static + Fn(&mut Cursive),
    {
        let cb = Callback::from_fn(cb);
        // We want to clone the Callback every time we call the closure
        let action = move |_: &mut T, _: &Event| {
            Some(EventResult::Consumed(Some(cb.clone())))
        };

        self.set_on_pre_event_inner(trigger, action);
    }

    /// Registers a callback when the given event is received.
    ///
    /// The given callback will be run before the child view sees the event.
    ///
    /// * If the result is `None`, then the child view is given the event as
    ///   usual.
    /// * Otherwise, it bypasses the child view and directly processes the
    ///   result.
    pub fn set_on_pre_event_inner<F, E>(&mut self, trigger: E, cb: F)
    where
        E: Into<EventTrigger>,
        F: Fn(&mut T, &Event) -> Option<EventResult> + 'static,
    {
        self.callbacks.push((
            trigger.into(),
            Action {
                phase: TriggerPhase::BeforeChild,
                callback: Rc::new(Box::new(cb)),
            },
        ));
    }

    /// Registers a callback when the given event is ignored by the child.
    ///
    /// If the child view ignores the event, `cb` will be called with the
    /// child view as argument.
    /// If the result is not `None`, it will be processed as well.
    pub fn set_on_event_inner<F, E>(&mut self, trigger: E, cb: F)
    where
        E: Into<EventTrigger>,
        F: Fn(&mut T, &Event) -> Option<EventResult> + 'static,
    {
        self.callbacks.push((
            trigger.into(),
            Action {
                phase: TriggerPhase::AfterChild,
                callback: Rc::new(Box::new(cb)),
            },
        ));
    }

    /// Remove any callbacks defined for this view.
    pub fn clear_callbacks(&mut self) {
        self.callbacks.clear();
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for OnEventView<T> {
    wrap_impl!(self.view: T);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        // Until we have better closure capture, define captured members separately.
        let callbacks = &self.callbacks;
        let view = &mut self.view;

        // * First, check all pre-child callbacks. Combine them.
        //   If any gets triggered and returns Some(...), stop right there.
        // * Otherwise, give the event to the child view.
        //   If it returns EventResult::Consumed, stop right there.
        // * Finally, check all post-child callbacks. Combine them.
        //   And just return the result.

        // First step: check pre-child
        callbacks
            .iter()
            .filter(|&(_, action)| action.phase == TriggerPhase::BeforeChild)
            .filter(|&(trigger, _)| trigger.apply(&event))
            .filter_map(|(_, action)| (*action.callback)(view, &event))
            .fold(None, |s, r| match s {
                // Return `Some()` if any pre-callback was present.
                None => Some(r),
                Some(c) => Some(c.and(r)),
            })
            .unwrap_or_else(|| {
                // If it was None, it means no pre-callback was triggered.
                // So let's give the view a chance!
                view.on_event(event.clone())
            })
            .or_else(|| {
                // No pre-child, and the child itself ignored the event?
                // Let's have a closer look then, shall we?
                callbacks
                    .iter()
                    .filter(|&(_, action)| {
                        action.phase == TriggerPhase::AfterChild
                    })
                    .filter(|&(trigger, _)| trigger.apply(&event))
                    .filter_map(|(_, action)| (*action.callback)(view, &event))
                    .fold(EventResult::Ignored, EventResult::and)
            })
    }
}
