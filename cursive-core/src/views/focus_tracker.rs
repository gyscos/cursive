use crate::{
    direction::Direction,
    event::{Event, EventResult},
    view::{CannotFocus, View, ViewWrapper},
    With,
};

/// Detects focus events for a view.
pub struct FocusTracker<T> {
    view: T,
    on_focus_lost: Box<dyn FnMut(&mut T) -> EventResult + Send + Sync>,
    on_focus: Box<dyn FnMut(&mut T) -> EventResult + Send + Sync>,
}

impl<T: Send + Sync + 'static> FocusTracker<T> {
    /// Wraps a view in a new `FocusTracker`.
    pub fn new(view: T) -> Self {
        FocusTracker {
            view,
            on_focus_lost: Box::new(|_| EventResult::Ignored),
            on_focus: Box::new(|_| EventResult::Ignored),
        }
    }

    /// Sets a callback to be run when the focus is gained.
    #[must_use]
    pub fn on_focus<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T) -> EventResult + Send + Sync,
    {
        self.with(|s| s.set_on_focus(f))
    }

    /// Sets a callback to be run when the focus is gained.
    #[crate::callback_helpers]
    pub fn set_on_focus<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T) -> EventResult + Send + Sync,
    {
        self.on_focus = Box::new(f);
    }

    /// Sets a callback to be run when the focus is lost.
    #[must_use]
    pub fn on_focus_lost<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T) -> EventResult + Send + Sync,
    {
        self.with(|s| s.set_on_focus_lost(f))
    }

    /// Sets a callback to be run when the focus is lost.
    #[crate::callback_helpers]
    pub fn set_on_focus_lost<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T) -> EventResult + Send + Sync,
    {
        self.on_focus_lost = Box::new(f);
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for FocusTracker<T> {
    wrap_impl!(self.view: T);

    fn wrap_take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        match self.view.take_focus(source) {
            Ok(res) => Ok(res.and((self.on_focus)(&mut self.view))),
            Err(CannotFocus) => Err(CannotFocus),
        }
    }

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        let res = if let Event::FocusLost = event {
            (self.on_focus_lost)(&mut self.view)
        } else {
            EventResult::Ignored
        };
        res.and(self.view.on_event(event))
    }
}

#[crate::blueprint(FocusTracker::new(view))]
struct Blueprint {
    view: crate::views::BoxedView,

    on_focus: Option<_>,

    on_focus_lost: Option<_>,
}

crate::manual_blueprint!(with focus_tracker, |config, context| {
    let on_focus = context.resolve(&config["on_focus"])?;
    let on_focus_lost = context.resolve(&config["on_focus_lost"])?;

    Ok(move |view| {
        let mut tracker = FocusTracker::new(view);

        if let Some(on_focus) = on_focus {
            tracker.set_on_focus_cb(on_focus);
        }

        if let Some(on_focus_lost) = on_focus_lost {
            tracker.set_on_focus_lost_cb(on_focus_lost);
        }

        tracker
    })
});
