use crate::{backend, event::Event, theme, Cursive, Vec2};
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
    fn layout(&mut self) {
        let size = self.screen_size();
        self.siv.borrow_mut().layout(size);
    }

    // Process any backend-requiring calls accumulated by the Cursive root.
    fn process_pending_backend_calls(&mut self) {
        let calls = std::mem::take(&mut self.backend_calls);
        for call in calls {
            (call)(&mut *self.backend);
        }
    }

    fn draw(&mut self) {
        let sizes = self.screen().layer_sizes();
        if self.last_sizes != sizes {
            // TODO: Maybe we only need to clear if the _max_ size differs?
            // Or if the positions change?
            self.clear();
            self.last_sizes = sizes;
        }

        if self.needs_clear {
            self.backend
                .clear(self.current_theme().palette[theme::PaletteColor::Background]);
            self.needs_clear = false;
        }

        let size = self.screen_size();

        self.siv.borrow_mut().draw(size, &*self.backend);
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
                self.on_event(Event::Refresh);
                self.process_pending_backend_calls();
            }

            self.refresh();
        }

        if boring {
            self.sleep();
            self.boring_frame_count += 1;
        }
    }

    /// post_events asynchronously
    #[cfg(feature = "wasm")]
    pub async fn post_events_async(&mut self, received_something: bool) {
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
                self.on_event(Event::Refresh);
                self.process_pending_backend_calls();
            }

            self.refresh();
        }

        if boring {
            self.sleep_async().await;
            self.boring_frame_count += 1;
        }
    }

    fn sleep(&self) {
        std::thread::sleep(Duration::from_millis(INPUT_POLL_DELAY_MS));
    }

    #[cfg(feature = "wasm")]
    async fn sleep_async(&self) {
        use wasm_bindgen::prelude::*;
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let closure = Closure::new(move || {
                resolve.call0(&JsValue::null()).unwrap();
            }) as Closure<dyn FnMut()>;
            web_sys::window()
                .expect("window is None for sleep")
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    INPUT_POLL_DELAY_MS as i32,
                )
                .expect("should register timeout for sleep");
            closure.forget();
        });
        let js_future = wasm_bindgen_futures::JsFuture::from(promise);
        js_future.await.expect("should await sleep");
    }

    /// Refresh the screen with the current view tree state.
    pub fn refresh(&mut self) {
        self.boring_frame_count = 0;

        // Do we need to redraw every time?
        // Probably, actually.
        // TODO: Do we need to re-layout every time?
        self.layout();

        // TODO: Do we need to redraw every view every time?
        // (Is this getting repetitive? :p)
        self.draw();
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

    /// step asynchronously
    #[cfg(feature = "wasm")]
    pub async fn step_async(&mut self) -> bool {
        let received_something = self.process_events();
        self.post_events_async(received_something).await;
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

    /// Runs the event loop asynchronously.
    #[cfg(feature = "wasm")]
    pub async fn run_async(&mut self) {
        self.refresh();

        // And the big event loop begins!
        while self.is_running() {
            self.step_async().await;
        }
    }    
}
