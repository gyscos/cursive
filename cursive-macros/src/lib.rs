use proc_macro::TokenStream;

mod builder;

/// Generate two helper functions to help working with cursive recipes.
///
/// # Problem to solve
///
/// When writing cursive recipes, it is often necessary to load parameters or variables.
///
/// Some of these have simple types like `u64` or `String`, but in some cases we need to load a
/// callback.
///
/// In this case, the recipe loading the variable and the user storing the variable need
/// to use the exact same type, or downcasting will not work. This is made complicated by Rust's
/// closures, where each closure is a unique anonymous type: if the user directly stores a closure,
/// there will be no way to identify its exact type to downcast it in the recipe.
///
/// Instead, both sides (recipe and user) need to agree to use a fixed type, for example a trait
/// object like `Arc<dyn Fn(&mut Cursive)>` (we use `Arc` rather than `Box` because we want variables
/// to be cloneable).
///
/// It's a bit cumbersome having to write the exact type including the `Arc` whenever we want to
/// store a callback for a recipe. Similarly, it's a bit annoying when writing the recipe to make
/// sure the correct `Arc<...>` type is fetched and converted to a type directly usable as callback.
///
/// # Solution
///
/// This is where this macro comes into play: from an original function that uses a callback, it
/// generates two helper functions:
/// * A _maker_ function, to be used when storing variables. This function takes a generic type
/// implementing the same `Fn` trait as the desired callback, and returns it wrapped in the correct
/// trait object.
/// * A _setter_ function, to be used when writing recipes. This function wraps the original
/// function, but takes a trait-object instead of a generic `Fn` type, and unwraps it internally.
///
/// # Notes
///
/// * The wrapped function doesn't even have to take `self`, it can be a "static"
/// constructor method.
/// * The `maker` function always takes a `Fn`, not a `FnMut` or `FnOnce`.
/// Use the `cursive::immut1!` (and others) macros to wrap a `FnMut` if you need it.
///
/// # Examples
///
/// ```rust,ignore
/// struct Foo {
///     callback: Box<dyn Fn(&mut Cursive)>,
/// }
///
/// impl Foo {
///     #[cursive::callback_helpers]
///     pub fn new<F>(callback: F) -> Self
///     where
///         F: Fn(&mut Cursive) + 'static,
///     {
///         let callback = Box::new(callback);
///         Foo { callback }
///     }
/// }
///
/// cursive::recipe!(Foo, |config, context| {
///     let foo =
///         Foo::new_with_cb(context.resolve(config["callback"])?);
///
///     Ok(foo)
/// });
///
/// // Elsewhere
/// fn foo() {
///     let mut context = cursive::builder::Context::new();
///
///     context.store("callback", Foo::new_cb(|s| s.quit()));
/// }
/// ```
#[proc_macro_attribute]
pub fn callback_helpers(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    builder::callback_helpers(item)
}

/// Defines a recipe for creating a view from config.
#[proc_macro_attribute]
pub fn recipe(attrs: TokenStream, item: TokenStream) -> TokenStream {
    builder::recipe(attrs, item)
}
