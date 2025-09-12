//! Module for Rx-like watchers and pipes.
//!
//! # Examples
//!
//! ```rust
//! # use cursive_core::utils::Rx;
//!
//! let value = Rx::new("abc");
//! let other = Rx::new("");
//!
//! value.set_on_change(|val| println!("{val}"));
//! value.set_on_change({
//!     let other = other.clone();
//!     move |val| other.set(val)
//! });
//! value.pipe_to(&other);
//! ```
//!
//! # TODO
//!
//! * Detect update cycle?
//! * Run callbacks in separate thread?
//! * More flexible watchers?
//!     * See the before and after values
//!     * Give a chance to cancel the change
//! * Reduce risk of Arc cycle and memory leak
use std::{
    ops::Deref,
    sync::{Arc, Mutex, RwLock},
};

/// The shared innner representation of a family of `Rx`.
struct Inner<T> {
    // This looks like the usual Arc<Mutex> but reversed.
    // * `Inner` itself will usually be behind an `Arc.
    // * `Mutex` is used to get writeable access to the value.
    // * The `Arc` is there so we can keep a local copy of the value without a lock.
    //   (Useful when running the watchers on the value.)
    value: Mutex<Arc<T>>,

    // * Each watcher is wrapped in an `Arc` so it can be cloned.
    // * The Vec is wrapped in an `Arc` so we can get a local copy without keeping a lock.
    //   We use `Arc::make_mut` on that whenever needed. This requires the value to be Clone.
    // * That is behind a `RwLock` so we can get mutable access when needed.
    watchers: RwLock<Arc<Vec<Watcher<T>>>>,
}

type Watcher<T> = Arc<dyn Fn(&T) + Send + Sync>;

/// A reactive value.
///
/// It is a wrapper around a value, with callbacks triggered when the value changes.
///
/// This is all internally wrapped in an `Arc`, so multiple `Rx` can point to the same shared
/// value.
///
/// Recursive bindings are possible:
///
/// ```rust
/// use cursive_core::utils::Rx;
/// let rx = Rx::new(String::new());
/// rx.map(|c| c.len()).map(|len| format!("This sentence has {len} chars.")).pipe_to(&rx);
///
/// assert_eq!(rx.get(), String::from("This sentence has 27 chars."));
/// ```
pub struct Rx<T> {
    inner: Arc<Inner<T>>,
}

/// A buffered reactive value.
///
/// It is a wrapper around `Rx<T>` with a local buffer.
pub struct BRx<T> {
    rx: Rx<T>,
    buffer: Arc<T>,
}

impl<T> BRx<T>
where
    T: Clone + PartialEq,
{
    /// Creates a new buffered `Rx` around the given value.
    pub fn new(value: T) -> Self {
        let buffer = Arc::new(value.clone());
        let rx = Rx::new(value);
        Self { rx, buffer }
    }

    /// Wraps an existing `Rx`.
    ///
    /// Uses the current value as buffer.
    pub fn wrap(rx: Rx<T>) -> Self {
        let buffer = Arc::new(rx.get());
        Self { rx, buffer }
    }

    /// Refreshes the buffer with a clone of the current value.
    ///
    /// Returns `true` if the value changed since the last refresh.
    pub fn refresh(&mut self) -> bool {
        self.rx.call_on(|c| {
            if *c != *self.buffer {
                Arc::make_mut(&mut self.buffer).clone_from(c);
                true
            } else {
                false
            }
        })
    }

    /// Runs the given closure on both the buffer and the shared value.
    pub fn call_on_mut<F>(&mut self, f: F)
    where
        F: Fn(&mut T),
    {
        // Lock the value while we update it.
        self.rx.call_on_mut(|c| {
            let buffer = Arc::make_mut(&mut self.buffer);

            // The contract is that the buffer does not change under out nose.
            // So if the shared value had a change since the last refresh, too bad.
            // The local buffer wins.
            if *c != *buffer {
                c.clone_from(buffer);
            }

            f(buffer);
            f(c);

            // If the two function resulted in different results, fix that.
            if *c != *buffer {
                c.clone_from(buffer);
            }
        });
    }

    /// Sets a new shared value.
    ///
    /// The buffer will be cloned from this value as well.
    pub fn set(&mut self, value: T) {
        Arc::make_mut(&mut self.buffer).clone_from(&value);
        self.rx.set(value);
    }

    /// Returns a `Rx` sharing this content.
    pub fn rx(&self) -> Rx<T> {
        self.rx.clone()
    }

    /// Gets a ref to the current buffer.
    pub fn get(&self) -> &T {
        &self.buffer
    }

    /// Returns an Arc to the shared buffer.
    ///
    /// This avoids cloning the buffer in most situations.
    pub fn buffer(&self) -> Arc<T> {
        Arc::clone(&self.buffer)
    }
}

impl<T> Deref for BRx<T>
where
    T: Clone + PartialEq,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> Clone for Rx<T> {
    fn clone(&self) -> Self {
        let inner = Arc::clone(&self.inner);
        Self { inner }
    }
}

impl<T: Clone + PartialEq> Inner<T> {
    /// Create a new shared `Inner`.
    pub fn new(value: T) -> Self {
        Self {
            value: Mutex::new(Arc::new(value)),
            watchers: RwLock::new(Arc::new(Vec::new())),
        }
    }

    /// Adds a new callback, called whenever the value changes.
    pub fn set_on_change(&self, f: impl Fn(&T) + Send + Sync + 'static) {
        Arc::make_mut(&mut *self.watchers.write().unwrap()).push(Arc::new(f));
    }

    /// Sets a new shared value.
    ///
    /// If the value changed (according to PartialEq), then all callbacks will be triggered.
    pub fn set(&self, value: T) {
        // Only lock the value while we run the closure.
        let value = {
            let mut current = self.value.lock().unwrap();
            if value == **current {
                return;
            }

            *Arc::make_mut(&mut current) = value;
            Arc::clone(&current)
        };

        // Get a local copy of the watchers so we don't keep any lock.
        let watchers = Arc::clone(&self.watchers.read().unwrap());

        for watcher in watchers.iter() {
            watcher(&value);
        }
    }

    /// Runs a closure on the stored value, and return the result.
    pub fn call_on<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        f(&self.value.lock().unwrap())
    }

    pub fn call_on_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let r;

        // Only lock the value while we run the closure.
        let value = {
            let mut value = self.value.lock().unwrap();
            r = f(Arc::make_mut(&mut value));
            Arc::clone(&value)
        };

        // Get a local copy of the watchers so we don't keep any lock.
        let watchers = Arc::clone(&self.watchers.read().unwrap());

        for watcher in watchers.iter() {
            watcher(&value);
        }

        r
    }

    /// Remove all callback/watchers on this `Rx`.
    ///
    /// This will affect all `Rx` sharing the same value.
    ///
    /// Note that this will break any connection made with `Rx::map` or `Rx::connect_to`.
    pub fn clear_watchers(&self) {
        Arc::make_mut(&mut *self.watchers.write().unwrap()).clear();
    }

    // TODO: a method that returns a lockGuard<T?>
}

impl<T: Default + Clone + PartialEq> Default for Rx<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Clone + PartialEq> From<T> for Rx<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Rx<T>
where
    T: Clone + PartialEq,
{
    /// Creates a new `Rx` with the given value.
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(Inner::new(value)),
        }
    }

    /// Returns the receiver half of a channel with cloned values from `self`.
    ///
    /// Any time the value in `self` changes, a clone of the value will be sent to the channel.
    pub fn read_channel(&self) -> std::sync::mpsc::Receiver<T>
    where
        T: Send + Sync + 'static,
    {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.set_on_change(move |v| {
            sender.send(v.clone()).ok();
        });

        receiver
    }

    /// Adds a callback to run whenever the value in `self` changes.
    pub fn set_on_change(&self, f: impl Fn(&T) + Send + Sync + 'static) {
        self.inner.set_on_change(f);
    }

    /// Sets a new shared value.
    ///
    /// If the value is different from the previous one (according to PartialEq), then all
    /// callbacks will be triggered.
    pub fn set(&self, value: T) {
        self.inner.set(value);
    }

    /// Runs a closure on the inner shared value, and return the result.
    pub fn call_on<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.inner.call_on(f)
    }

    /// Runs a closure on the inner shared value with mutable access.
    ///
    /// The watcher callbacks will be called unconditionally after the closure returns.
    pub fn call_on_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.inner.call_on_mut(f)
    }

    /// Gets a clone of the current value.
    pub fn get(&self) -> T {
        self.call_on(T::clone)
    }

    /// Returns a new `Rx`, which will be updated by applying the given closure to any value from
    /// `self`.
    ///
    /// Any time `self` receives a new value, `f` will be called, and the result will be set on the
    /// returned `Rx`.
    pub fn map<F, U>(&self, f: F) -> Rx<U>
    where
        F: Fn(&T) -> U + Send + Sync + 'static,
        U: Clone + PartialEq + Send + Sync + 'static,
    {
        let val = self.call_on(&f);

        let other = Rx::new(val);
        let res = other.clone();
        self.set_on_change(move |v| {
            let val = f(v);
            other.set(val);
        });

        res
    }

    /// Update `other` any time `self` receives a new value.
    pub fn pipe_to(&self, other: &Self)
    where
        T: Send + Sync + 'static,
    {
        {
            let other = other.clone();
            self.set_on_change(move |v| {
                other.set(v.clone());
            });
        }

        // Seed with the current value
        other.set(self.get());
    }

    /// Remove all change callbacks from `self`.
    pub fn clear_watchers(&mut self) {
        self.inner.clear_watchers();
    }
}

impl<T> Rx<Option<T>>
where
    T: Clone + PartialEq,
{
    /// Update `other` any time `self` receives a `Some` value.
    pub fn pipe_some(&self, other: &Rx<T>)
    where
        T: Send + Sync + 'static,
    {
        {
            let other = other.clone();
            self.set_on_change(move |v| {
                if let Some(v) = v {
                    other.set(v.clone());
                }
            });
        }

        if let Some(v) = self.get() {
            other.set(v);
        }
    }
}
