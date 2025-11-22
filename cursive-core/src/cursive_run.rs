use crate::{backend, buffer, event, Cursive, Vec2};
use parking_lot::RwLock;
use std::borrow::{Borrow, BorrowMut};
use std::time::Duration;

// How long we wait between two empty input polls
const INPUT_POLL_DELAY_MS: u64 = 30;

/// Event loop runner for a cursive instance.
///
/// You can get one from `Cursive::runner`, then either call `.run()`, or
/// manually `.step()`.
///
/// The `C` type is usually either `Cursive` or `&mut Cursive`.
pub struct CursiveRunner<C> {
    siv: C,

    backend: Box<dyn backend::Backend>,
    buffer: RwLock<buffer::PrintBuffer>,

    boring_frame_count: u32,
    // Last layer sizes of the stack view.
    // If it changed, clear the screen.
    last_sizes: Vec<Vec2>,
}

impl<C> std::ops::Deref for CursiveRunner<C>
where
    C: Borrow<Cursive>,
{
    type Target = Cursive;

    fn deref(&self) -> &Cursive {
        self.siv.borrow()
    }
}

impl<C> std::ops::DerefMut for CursiveRunner<C>
where
    C: BorrowMut<Cursive>,
{
    fn deref_mut(&mut self) -> &mut Cursive {
        self.siv.borrow_mut()
    }
}

impl<C> CursiveRunner<C> {
    /// Creates a new cursive runner wrapper.
    pub fn new(siv: C, backend: Box<dyn backend::Backend>) -> Self {
        CursiveRunner {
            siv,
            backend,
            buffer: RwLock::new(buffer::PrintBuffer::new()),
            boring_frame_count: 0,
            last_sizes: Vec::new(),
        }
    }

    /// Returns the size of the screen, in characters.
    fn screen_size(&self) -> Vec2 {
        self.backend.screen_size()
    }

    /// Clean out the terminal and get back the wrapped object.
    pub fn into_inner(self) -> C {
        self.siv
    }
}

impl<C> CursiveRunner<C>
where
    C: BorrowMut<Cursive>,
{
    fn layout(&mut self, size: Vec2) {
        self.siv.borrow_mut().layout(size);
    }

    // Process any backend-requiring calls accumulated by the Cursive root.
    fn process_pending_backend_calls(&mut self) {
        let calls = std::mem::take(&mut self.backend_calls);
        for call in calls {
            (call)(&mut *self.backend);
        }
    }

    fn draw(&mut self, size: Vec2) {
        let sizes = self.screen().layer_sizes();
        if self.last_sizes != sizes {
            // TODO: Maybe we only need to clear if the _max_ size differs?
            // Or if the positions change?
            self.clear();
            self.last_sizes = sizes;
        }

        self.buffer.write().resize(size);
        self.siv.borrow_mut().draw(&self.buffer);
        self.buffer.write().flush(&*self.backend);
    }

    /// Performs the first half of `Self::step()`.
    ///
    /// This is an advanced method for fine-tuned manual stepping;
    /// you probably want [`run`][1] or [`step`][2].
    ///
    /// This processes any pending event or callback. After calling this,
    /// you will want to call [`post_events`][3] with the result from this
    /// function.
    ///
    /// Returns `true` if an event or callback was received,
    /// and `false` otherwise.
    ///
    /// [1]: CursiveRunner::run()
    /// [2]: CursiveRunner::step()
    /// [3]: CursiveRunner::post_events()
    pub fn process_events(&mut self) -> bool {
        // Things are boring if nothing significant happened.
        let mut boring = true;

        // First, handle all available input
        while let Some(event) = self.backend.poll_event() {
            boring = false;
            self.on_event(event);
            self.process_pending_backend_calls();

            if !self.is_running() {
                return true;
            }
        }

        // Then, handle any available callback
        while self.process_callback() {
            boring = false;

            if !self.is_running() {
                return true;
            }
        }

        !boring
    }

    /// Performs the second half of `Self::step()`.
    ///
    /// This is an advanced method for fine-tuned manual stepping;
    /// you probably want [`run`][1] or [`step`][2].
    ///
    /// You should call this after [`process_events`][3].
    ///
    /// [1]: CursiveRunner::run()
    /// [2]: CursiveRunner::step()
    /// [3]: CursiveRunner::process_events()
    pub fn post_events(&mut self, received_something: bool) {
        let boring = !received_something;
        // How many times should we try if it's still boring?
        // Total duration will be INPUT_POLL_DELAY_MS * repeats
        // So effectively fps = 1000 / INPUT_POLL_DELAY_MS / repeats
        if !boring
            || self
                .fps()
                .map(|fps| 1000 / INPUT_POLL_DELAY_MS as u32 / fps.get())
                .map(|repeats| self.boring_frame_count >= repeats)
                .unwrap_or(false)
        {
            // We deserve to draw something!

            if boring {
                // We're only here because of a timeout.
                self.on_event(event::Event::Refresh);
                self.process_pending_backend_calls();
            }

            self.refresh();
        }

        if boring {
            std::thread::sleep(Duration::from_millis(INPUT_POLL_DELAY_MS));
            self.boring_frame_count += 1;
        }
    }

    /// Refresh the screen with the current view tree state.
    pub fn refresh(&mut self) {
        self.boring_frame_count = 0;

        // Capture screen size once during a refresh to
        // ensure layout and draw receive the same screen
        // size, otherwise bad things can happen.
        let screen_size = self.screen_size();

        // Do we need to redraw every time?
        // Probably, actually.
        // TODO: Do we need to re-layout every time?
        self.layout(screen_size);

        // TODO: Do we need to redraw every view every time?
        // (Is this getting repetitive? :p)
        self.draw(screen_size);
        self.backend.refresh();
    }

    /// Return the name of the backend used.
    ///
    /// Mostly used for debugging.
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    /// Performs a single step from the event loop.
    ///
    /// Useful if you need tighter control on the event loop.
    /// Otherwise, [`run(&mut self)`] might be more convenient.
    ///
    /// Returns `true` if an input event or callback was received
    /// during this step, and `false` otherwise.
    ///
    /// [`run(&mut self)`]: #method.run
    pub fn step(&mut self) -> bool {
        let received_something = self.process_events();
        self.post_events(received_something);
        received_something
    }

    /// Runs the event loop.
    ///
    /// It will wait for user input (key presses)
    /// and trigger callbacks accordingly.
    ///
    /// Internally, it calls [`step(&mut self)`] until [`quit(&mut self)`] is
    /// called.
    ///
    /// After this function returns, you can call it again and it will start a
    /// new loop.
    ///
    /// [`step(&mut self)`]: #method.step
    /// [`quit(&mut self)`]: #method.quit
    pub fn run(&mut self) {
        self.refresh();

        // And the big event loop begins!
        while self.is_running() {
            self.step();
        }
    }
}

#[cfg(test)]
mod test_layout_draw_refresh {
    use super::*;
    use crate::backend::Backend;
    use crate::event::Event;
    use crate::views::{EditView, Panel};
    use crate::{style, Vec2};
    use std::sync::{Arc, Mutex};

    // WigglyBackend is a Mock backend that simulates a previous
    // race condition by returning different sizes on successive
    // calls to screen_size().
    struct WigglyBackend {
        // call_count is a simple ounter for how many times
        // screen_size() has been called.
        call_count: Arc<Mutex<usize>>,
        // sizes are the sizes to return on successive calls
        // to screen_size(). We just pop a size off each time
        // it gets called.
        sizes: Vec<Vec2>,
    }

    impl WigglyBackend {
        fn new(sizes: Vec<Vec2>) -> Self {
            WigglyBackend {
                call_count: Arc::new(Mutex::new(0)),
                sizes,
            }
        }

        // shinking creates a backend that simulates
        // shrinking: first call returns 100, second returns 98.
        fn shrinking() -> Self {
            Self::new(vec![Vec2::new(100, 30), Vec2::new(98, 30)])
        }

        // growing creates a backend that simulates
        // growing: first call returns 98, second returns 100.
        fn growing() -> Self {
            Self::new(vec![Vec2::new(98, 30), Vec2::new(100, 30)])
        }
    }

    // And now make WigglyBackend a Backend.
    impl Backend for WigglyBackend {
        fn name(&self) -> &str {
            "WigglyBackend"
        }

        fn screen_size(&self) -> Vec2 {
            let mut count = self.call_count.lock().unwrap();
            let idx = *count;
            *count += 1;

            // And here's where we return different sizes on
            // successive calls to simulate potential race
            // condition issues if the bug is re-introduced
            // and/or screen_size() starts being called twice
            // in the same refresh cycle again.
            self.sizes.get(idx).copied().unwrap_or_else(|| {
                // ..if we've returned all of the sizes,
                // just keep returning the last size.
                *self.sizes.last().unwrap()
            })
        }

        // poll_event is nused, but part of the trait.
        fn poll_event(&mut self) -> Option<Event> {
            None
        }

        // set_title is unused, but part of the trait.
        fn set_title(&mut self, _title: String) {}

        // refresh is unused, but part of the trait.
        fn refresh(&mut self) {}

        // has_colors is unused, but part of the trait.
        fn has_colors(&self) -> bool {
            false
        }

        // move_to is unused, but part of the trait.
        fn move_to(&self, _pos: Vec2) {}

        // print is unused, but part of the trait.
        fn print(&self, _text: &str) {}

        // clear is unused, but part of the trait.
        fn clear(&self, _color: style::Color) {}

        // set_color unused, but part of the trait.
        fn set_color(&self, colors: style::ColorPair) -> style::ColorPair {
            colors
        }

        // set_effect is unused, but part of the trait.
        fn set_effect(&self, _effect: style::Effect) {}

        // unset_effecct is unnused, but part of the trait.
        fn unset_effect(&self, _effect: style::Effect) {}
    }

    #[test]
    // Test to make sure the window can shrink and EditView won't panic.
    fn test_shrink_race_condition() {
        let mut siv = Cursive::new();

        // Add EditView in Panel, which would be where the panic
        // triggers if screen_size() is being called multiple times
        // in a refresh cycle.
        siv.add_layer(Panel::new(
            EditView::new().content("i exist to stay calm and not panic when shrinking"),
        ));

        // Create a backend that returns 100 on first call, 98 on second call.
        // If screen_size() gets called twice during a refresh, this would
        // trigger a panic in EditView.
        let backend = Box::new(WigglyBackend::shrinking());
        let mut runner = CursiveRunner::new(siv, backend);
        runner.refresh();
    }

    #[test]
    // Test to make sure the window can grow and EditView won't panic.
    fn test_grow_race_condition() {
        let mut siv = Cursive::new();

        siv.add_layer(Panel::new(EditView::new().content("i exist to stay calm and not panic when growing")));

        // Backend returns 98 on first call, 100 on second call.
        let backend = Box::new(WigglyBackend::growing());
        let mut runner = CursiveRunner::new(siv, backend);
        runner.refresh();
    }

    #[test]
    // Verifies that screen_size() is only called once for a given refresh cycle.
    fn test_screen_size_called_once() {
        let mut siv = Cursive::new();

        siv.add_layer(Panel::new(EditView::new().content("i exist to test")));

        let backend = Box::new(WigglyBackend::shrinking());
        let call_count = backend.call_count.clone();
        let mut runner = CursiveRunner::new(siv, backend);

        runner.refresh();

        let count = *call_count.lock().unwrap();
        assert_eq!(
            count, 1,
            "screen_size() should only be called once per refresh(), but was called {} times",
            count
        );
    }

    #[test]
    // Verifies that layout() and draw() receive the same size.
    fn test_layout_draw_size_consistency() {
        let mut siv = Cursive::new();

        let layout_size = Arc::new(Mutex::new(None));
        let draw_size = Arc::new(Mutex::new(None));

        struct SizeRecorder {
            layout_size: Arc<Mutex<Option<Vec2>>>,
            draw_size: Arc<Mutex<Option<Vec2>>>,
        }

        impl crate::view::View for SizeRecorder {
            fn layout(&mut self, size: Vec2) {
                *self.layout_size.lock().unwrap() = Some(size);
            }

            fn draw(&self, printer: &crate::Printer) {
                *self.draw_size.lock().unwrap() = Some(printer.size);
            }
        }

        siv.add_layer(Panel::new(SizeRecorder {
            layout_size: layout_size.clone(),
            draw_size: draw_size.clone(),
        }));

        // Use the wiggly backend that would return different sizes
        // if we asked for the screen size twice in the same
        // refresh cycle.
        let backend = Box::new(WigglyBackend::shrinking());
        let mut runner = CursiveRunner::new(siv, backend);

        runner.refresh();

        let layout = layout_size.lock().unwrap().unwrap();
        let draw = draw_size.lock().unwrap().unwrap();

        assert_eq!(
            layout, draw,
            "layout and draw must always get the same size. Got layout={:?}, draw={:?}.",
            layout, draw
        );
    }
}
