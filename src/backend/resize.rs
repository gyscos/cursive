use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use signal_hook::iterator::Signals;

use backend::InputRequest;
use event::Event;

/// This starts a new thread to listen for SIGWINCH signals
///
/// As long as `resize_running` is true, it will listen for SIGWINCH, and,
/// when detected, it wil set `needs_resize` to true and send an event to
/// `resize_sender`. It will also consume an event from `resize_requests`
/// afterward, to keep the balance in the force.
#[cfg(unix)]
pub fn start_resize_thread(
    signals: Signals, resize_sender: Sender<Option<Event>>,
    resize_requests: Receiver<InputRequest>, resize_running: Arc<AtomicBool>,
    needs_resize: Option<Arc<AtomicBool>>,
) {
    thread::spawn(move || {
        // This thread will listen to SIGWINCH events and report them.
        while resize_running.load(Ordering::Relaxed) {
            // We know it will only contain SIGWINCH signals, so no need to check.
            if signals.wait().count() > 0 {
                // Tell ncurses about the new terminal size.
                // Well, do the actual resizing later on, in the main thread.
                // Ncurses isn't really thread-safe so calling resize_term() can crash
                // other calls like clear() or refresh().
                if let Some(ref needs_resize) = needs_resize {
                    needs_resize.store(true, Ordering::Relaxed);
                }

                resize_sender.send(Some(Event::WindowResize)).unwrap();
                // We've sent the message.
                // This means Cursive was listening, and will now soon be sending a new request.
                // This means the input thread accepted a request, but hasn't sent a message yet.
                // So we KNOW the input thread is not waiting for a new request.

                // We sent an event for free, so pay for it now by consuming a request
                while let Ok(InputRequest::Peek) = resize_requests.recv() {
                    // At this point Cursive will now listen for input.
                    // There is a chance the input thread will send his event before us.
                    // But without some extra atomic flag, it'd be hard to know.
                    // So instead, keep sending `None`

                    // Repeat until we receive a blocking call
                    resize_sender.send(None).unwrap();
                }
            }
        }
    });
}
