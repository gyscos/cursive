use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::Sender;
use signal_hook::iterator::Signals;

/// This starts a new thread to listen for SIGWINCH signals
#[allow(unused)]
pub fn start_resize_thread(resize_sender: Sender<()>, resize_running: Arc<AtomicBool>) {
    let mut signals = Signals::new([libc::SIGWINCH]).unwrap();
    thread::spawn(move || {
        // This thread will listen to SIGWINCH events and report them.
        while resize_running.load(Ordering::Relaxed) {
            // We know it will only contain SIGWINCH signals, so no need to check.
            if signals.wait().count() > 0 && resize_sender.send(()).is_err() {
                return;
            }
        }
    });
}
