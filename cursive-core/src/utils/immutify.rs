/*

// TODO: replace the 3 macros with 3 functions once they work correctly with
// reference arguments.
// (The returned closure must implement the for<'a> Fn(T, U<'a>)...


// TODO: replace the 3 macros/functions with a generic function when it can
// accept any number of arguments.

/// Wraps a `FnMut` into a `Fn`
///
/// This can be used to use a `FnMut` when a callback expects a `Fn`.
///
/// # Note
///
/// If the resulting `Fn` is called recursively, subsequent calls will be
/// no-ops.
pub fn immutify<F: FnMut(&mut Cursive)>(
    f: F,
) -> impl for<'s> Fn(&'s mut Cursive) {
    let callback = RefCell::new(f);
    move |s| {
        // Here's the weird trick: if we're already borrowed,
        // just ignored the callback.
        if let Ok(mut f) = callback.try_borrow_mut() {
            // Beeeaaah that's ugly.
            // Why do we need to manually dereference here?
            (&mut *f)(s);
        }
    }
}

*/

/// Macro to wrap a `FnMut` with 1 argument into a `Fn`.
///
/// This can wrap any `FnMut` with a single arguments (for example `&mut Cursive`).
///
/// See [`immut2!`] and [`immut3!`] to support a different number of arguments.
///
/// [`immut2!`]: crate::immut2!
/// [`immut3!`]: crate::immut3!
///
/// # Note
///
/// If this function tries to call itself recursively (for example by
/// triggering an event in `Cursive`), the second call will be a no-op.
/// Enabling recursive calls would break the `FnMut` contract.
///
/// In addition, due to weird interaction between Higher-rank trait bounds and
/// closures, you should use the result from the macro directly, and not
/// assign it to a variable.
///
/// # Examples
///
/// ```rust
/// # use cursive_core::{Cursive, immut1};
/// # fn main() {
/// # let mut siv = Cursive::new();
/// let mut i = 0;
/// // `Cursive::add_global_callback` takes a `Fn(&mut Cursive)`
/// siv.add_global_callback(
///     'q',
///     immut1!(move |s: &mut Cursive| {
///         // But here we mutate the environment! Crazy!
///         i += 1;
///         if i == 5 {
///             s.quit();
///         }
///     }),
/// );
/// # }
/// ```
#[macro_export]
macro_rules! immut1 {
    ($f:expr ; else $else:expr) => {{
        let callback = ::std::sync::Mutex::new($f);
        move |s| {
            if let ::std::result::Result::Ok(mut f) = callback.try_lock() {
                (&mut *f)(s)
            } else {
                $else
            }
        }
    }};
    ($f:expr) => {{
        let callback = ::std::sync::Mutex::new($f);
        move |s| {
            if let ::std::result::Result::Ok(mut f) = callback.try_lock() {
                (&mut *f)(s);
            }
        }
    }};
}

/// Macro to wrap a `FnOnce` with 1 argument into a `FnMut`.
///
/// This can wrap any `FnOnce` with a single argument (for example `&mut Cursive`).
///
/// # Note
///
/// If the resulting function is called multiple times, only the first call will trigger the
/// wrapped `FnOnce`. All subsequent calls will be no-ops.
///
/// In addition, due to weird interaction between Higher-rank trait bounds and
/// closures, you should use the result from the macro directly, and not
/// assign it to a variable.
#[macro_export]
macro_rules! once1 {
    ($f:expr) => {{
        let mut callback = ::std::option::Option::Some($f);
        move |s| {
            if let ::std::option::Option::Some(f) = callback.take() {
                f(s);
            }
        }
    }};
}

/// Macro to wrap a `FnMut` with 2 arguments into a `Fn`.
///
/// This can wrap any `FnMut` with two arguments.
///
/// See [`immut1!`] and [`immut3!`] to support a different number of arguments.
///
/// [`immut1!`]: crate::immut1!
/// [`immut3!`]: crate::immut3!
///
/// # Note
///
/// If this function tries to call itself recursively (for example by
/// triggering an event in `Cursive`), the second call will be a no-op.
/// Enabling recursive calls would break the `FnMut` contract.
///
/// In addition, due to weird interaction between Higher-rank trait bounds and
/// closures, you should use the result from the macro directly, and not
/// assign it to a variable.
#[macro_export]
macro_rules! immut2 {
    ($f:expr ; else $else:expr) => {{
        let callback = ::std::sync::Mutex::new($f);
        move |s, t| {
            if let ::std::result::Result::Ok(mut f) = callback.try_lock() {
                (&mut *f)(s, t)
            } else {
                $else
            }
        }
    }};
    ($f:expr) => {{
        let callback = ::std::sync::Mutex::new($f);
        move |s, t| {
            if let Ok(mut f) = callback.try_lock() {
                (&mut *f)(s, t);
            }
        }
    }};
}

/// Macro to wrap a `FnMut` with 3 arguments into a `Fn`.
///
/// This can wrap any `FnMut` with three arguments.
///
/// See [`immut1!`] and [`immut2!`] to support a different number of arguments.
///
/// [`immut1!`]: crate::immut1!
/// [`immut2!`]: crate::immut2!
///
/// # Note
///
/// If this function tries to call itself recursively (for example by
/// triggering an event in `Cursive`), the second call will be a no-op.
/// Enabling recursive calls would break the `FnMut` contract.
///
/// In addition, due to weird interaction between Higher-rank trait bounds and
/// closures, you should use the result from the macro directly, and not
/// assign it to a variable.
#[macro_export]
macro_rules! immut3 {
    ($f:expr ; else $else:expr) => {{
        let callback = ::std::sync::Mutex::new($f);
        move |s, t, u| {
            if let ::std::result::Result::Ok(mut f) = callback.try_lock() {
                (&mut *f)(s, t, u)
            } else {
                $else
            }
        }
    }};
    ($f:expr) => {{
        let callback = ::std::sync::Mutex::new($f);
        move |s, t, u| {
            if let Ok(mut f) = callback.try_lock() {
                (&mut *f)(s, t, u);
            }
        }
    }};
}

#[cfg(test)]
mod tests {

    fn call(f: impl Fn(i64, i64, i64), a: i64, b: i64, c: i64) {
        f(a, b, c)
    }

    #[test]
    fn immut3_loop() {
        let mut count = 0;
        let reenter: &mut dyn FnMut(i64, i64, i64) = &mut |a, b, c| count += a + b + c;
        for _i in 0..3 {
            call(immut3! { |a, b, c| reenter(2*a, b, c) }, 1, 2, 3);
        }
        assert_eq!(count, 7 * 3);
    }
}
