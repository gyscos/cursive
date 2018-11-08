use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Atomic counter used by [`ProgressBar`].
///
/// [`ProgressBar`]: ../views/struct.ProgressBar.html
#[derive(Clone, Debug)]
pub struct Counter(pub Arc<AtomicUsize>);

impl Counter {
    /// Creates a new `Counter` starting with the given value.
    pub fn new(value: usize) -> Self {
        Counter(Arc::new(AtomicUsize::new(value)))
    }

    /// Retrieves the current progress value.
    pub fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    /// Sets the current progress value.
    pub fn set(&self, value: usize) {
        self.0.store(value, Ordering::Relaxed);
    }

    /// Increase the current progress by `ticks`.
    pub fn tick(&self, ticks: usize) {
        self.0.fetch_add(ticks, Ordering::Relaxed);
    }
}
