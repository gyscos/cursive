//! Build views from configuration.
//!
//! ## Recipes
//!
//! Recipes define how to build a view from a json-like config object.
//!
//! It should be easy for third-party view to define a recipe.
//!
//! ## Builders
//!
//! * Users can prepare a builder `Context` to build views, which will collect all available recipes.
//! * They can optionally store named "variables" in the context (callbacks, sizes, ...).
//! * They can then load a configuration (often a yaml file) and render the view in there.
//!
//! ## Details
//!
//! This crate includes:
//! - A public part, always enabled.
//! - An implementation module, conditionally compiled.
#![cfg_attr(not(feature = "builder"), allow(unused))]
use crate::views::BoxedView;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use std::any::Any;

type MakerTrait<T> = dyn Fn(&Config, &Context) -> Result<T, Error> + Send + Sync;

/// Type of a trait-object that can build.
///
/// Stored as variables.
type Maker<T> = Box<MakerTrait<T>>;

type AnyMaker = Maker<Box<dyn Any>>;

/// Type of a config item.
pub type Config = serde_json::Value;

/// Type of a config object.
pub type Object = serde_json::Map<String, serde_json::Value>;

/// Can build a view from a config.
pub type BareBuilder = fn(&serde_json::Value, &Context) -> Result<BoxedView, Error>;

/// Boxed builder
type BoxedBuilder = Box<dyn Fn(&Config, &Context) -> Result<BoxedView, Error> + Send + Sync>;

/// Can build a wrapper from a config.
pub type BareWrapperBuilder = fn(&serde_json::Value, &Context) -> Result<Wrapper, Error>;

/// Boxed wrapper builder
type BoxedWrapperBuilder =
    Box<dyn Fn(&serde_json::Value, &Context) -> Result<Wrapper, Error> + Send + Sync>;

/// Can wrap a view.
pub type Wrapper = Box<dyn FnOnce(BoxedView) -> BoxedView + Send + Sync>;

/// Can build a callback
pub type BareVarBuilder = fn(&serde_json::Value, &Context) -> Result<Box<dyn Any>, Error>;

/// Boxed variable builder
///
/// If you store a variable of this type, when loading type `T`, it will run
/// this builder and try to downcast the result to `T`.
pub type BoxedVarBuilder =
    Arc<dyn Fn(&serde_json::Value, &Context) -> Result<Box<dyn Any>, Error> + Send + Sync>;

/// Everything needed to prepare a view from a config.
/// - Current recipes
/// - Any stored variables/callbacks
#[derive(Clone)]
pub struct Context {
    // TODO: Merge variables and recipes?
    // TODO: Use RefCell? Or even Arc<Mutex>?
    // So we can still modify the context when sub-context are alive.
    variables: Arc<Variables>,
    recipes: Arc<Recipes>,
}

struct Recipes {
    recipes: HashMap<String, BoxedBuilder>,
    wrappers: HashMap<String, BoxedWrapperBuilder>,
    parent: Option<Arc<Recipes>>,
}

/// Wrapper around a value that makes it Cloneable, but can only be resolved once.
pub struct ResolveOnce<T>(std::sync::Arc<std::sync::Mutex<Option<T>>>);

/// Return a variable-maker (for use in store_with)
pub fn resolve_once<T>(value: T) -> impl Fn(&Config, &Context) -> Result<T, Error>
where
    T: Send,
{
    let value = Mutex::new(Some(value));
    move |_, _| {
        value
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| Error::MakerFailed("variable was already resolved".to_string()))
    }
}

impl<T> ResolveOnce<T> {
    /// Take the value from self.
    pub fn take(&self) -> Option<T> {
        self.0.lock().unwrap().take()
    }

    /// Check if there is a value still to be resolved in self.
    pub fn is_some(&self) -> bool {
        self.0.lock().unwrap().is_some()
    }
}

impl Recipes {
    fn build(&self, name: &str, config: &Config, context: &Context) -> Result<BoxedView, Error> {
        if let Some(recipe) = self.recipes.get(name) {
            (recipe)(config, context).map_err(|e| Error::RecipeFailed(name.into(), Box::new(e)))
        } else {
            match self.parent {
                Some(ref parent) => parent.build(name, config, context),
                None => Err(Error::RecipeNotFound(name.into())),
            }
        }
    }

    fn build_wrapper(
        &self,
        name: &str,
        config: &Config,
        context: &Context,
    ) -> Result<Wrapper, Error> {
        if let Some(recipe) = self.wrappers.get(name) {
            (recipe)(config, context).map_err(|e| Error::RecipeFailed(name.into(), Box::new(e)))
        } else {
            match self.parent {
                Some(ref parent) => parent.build_wrapper(name, config, context),
                None => Err(Error::RecipeNotFound(name.into())),
            }
        }
    }
}

enum VarEntry {
    // Proxy variable used for sub-templates
    Proxy(Arc<String>),

    // Regular variable set by user or recipe
    //
    // Set by user:
    //  - Clone-able
    //  -
    Maker(AnyMaker),

    // Embedded config by intermediate node
    Config(Config),
    // Optional: store recipes separately?
}

impl VarEntry {
    fn proxy(var_name: impl Into<String>) -> Self {
        VarEntry::Proxy(Arc::new(var_name.into()))
    }

    fn maker(maker: AnyMaker) -> Self {
        VarEntry::Maker(maker)
    }

    fn config(config: impl Into<Config>) -> Self {
        VarEntry::Config(config.into())
    }
}

struct Variables {
    variables: HashMap<String, VarEntry>,

    // If something is not found in this scope, try the next one!
    parent: Option<Arc<Variables>>,
}

/// Error during config parsing.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The configuration was invalid.
    InvalidConfig {
        /// Description of the issue
        message: String,

        /// Optional offending config object
        config: Config,
    },

    /// Found no variable with the given name.
    NoSuchVariable(String),

    /// Found a variable, but with a different type than expected.
    IncorrectVariableType {
        /// Name of the offending variable
        name: String,
        /// Expected type
        expected_type: String,
    },

    /// Could not load the given config.
    CouldNotLoad {
        /// Expected type
        expected_type: String,

        /// Config value that could not be parsed
        config: Config,
    },

    /// A recipe was not found
    RecipeNotFound(String),

    /// A recipe failed to run.
    ///
    /// This is in direct cause to an error in an actual recipe.
    RecipeFailed(String, Box<Error>),

    /// A maker failed to produce a value.
    MakerFailed(String),

    /// We failed to resolve a value.
    ///
    /// It means we failed to load it as a variable, and also failed to load it as a config.
    ResolveFailed {
        /// Failure while resolving the value as a variable.
        var_failure: Box<Error>,

        /// Failure while resolving the value as a config.
        config_failure: Box<Error>,
    },
}

impl Error {
    /// Convenient method to create an error from a message and a problematic config.
    pub fn invalid_config<S: Into<String>, C: Clone + Into<Config>>(
        message: S,
        config: &C,
    ) -> Self {
        let message = message.into();
        let config = config.clone().into();
        Error::InvalidConfig { message, config }
    }
}

/// Error caused by an invalid config.
#[derive(Debug)]
pub struct ConfigError {
    /// Variable names present more than once in the config.
    ///
    /// Each variable can only be read once.
    pub duplicate_vars: HashSet<String>,

    /// Variable names not registered in the context before loading the config.
    pub missing_vars: HashSet<String>,
}

impl ConfigError {
    /// Creates a config error if any issue is detected.
    fn from(duplicate_vars: HashSet<String>, missing_vars: HashSet<String>) -> Result<(), Self> {
        if duplicate_vars.is_empty() && missing_vars.is_empty() {
            Ok(())
        } else {
            Err(Self {
                duplicate_vars,
                missing_vars,
            })
        }
    }
}

// Parse the given config, and yields all the variable names found.
fn inspect_variables<F: FnMut(&str)>(config: &Config, on_var: &mut F) {
    match config {
        Config::String(name) => {
            if let Some(name) = name.strip_prefix('$') {
                on_var(name);
            }
        }
        Config::Array(array) => {
            for value in array {
                inspect_variables(value, on_var);
            }
        }
        Config::Object(object) => {
            for value in object.values() {
                inspect_variables(value, on_var);
            }
        }
        _ => (),
    }
}

/// Trait for types that can be resolved from a context.
///
/// They can be loaded from a config (yaml), or from a stored value (`Box<Any>`).
pub trait Resolvable {
    /// Build from a config (a JSON value).
    ///
    /// The default implementation always fails.
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::CouldNotLoad {
            expected_type: std::any::type_name::<Self>().to_string(),
            config: config.clone(),
        })
    }

    /// Build from an `Any` variable.
    ///
    /// Default implementation tries to downcast to `Self`.
    ///
    /// Override if you want to try downcasting to other types as well.
    fn from_any(any: Box<dyn Any>) -> Option<Self>
    where
        Self: Sized + Any,
    {
        any.downcast().ok().map(|b| *b)
    }
}

// Implement a trait for Fn(A, B), Fn(&A, B), Fn(A, &B), ...
// We do this by going down a tree:
// (D C B A)
//      (C B A) to handle the case for 3 args
//          ...
//      <> [] (D C B A) to start actual work for 4 args
//          <D> [D]  (C B A)        |
//          <D: ?Sized> [&D] (C B A)        | Here we branch and recurse
//          <D: ?Sized> [&mut D] (C B A)    |
//              ...
//              <A B C D> [A B C D]  ()          |
//              ...                              |
//              <A B C: ?Sized D> [A B &C D] ()  | Final implementations
//              ...                              |
//              <A: ?Sized B: ?Sized C: ?Sized D: ?Sized> [&mut A &mut B &mut C &mut D] ()
macro_rules! impl_fn_from_config {
    // Here is a graceful end for recursion.
    (
        $trait:ident
        ( )
    ) => { };
    (
        $trait:ident
        < $($letters:ident $(: ?$unbound:ident)?)* >
        [ $($args:ty)* ]
        ( )
    ) => {
        // The leaf node is the actual implementation
        #[allow(coherence_leak_check)]
        impl<Res, $($letters $(: ?$unbound)?),* > $trait for Arc<dyn Fn($($args),*) -> Res + Send + Sync> {}
    };
    (
        $trait:ident
        < $($letters:ident $(: ?$unbound:ident)?)* >
        [ $($args:ty)* ]
        ( $head:ident $($leftover:ident)* )
    ) => {
        // Here we just branch per ref type
        impl_fn_from_config!(
            $trait
            < $head $($letters $(: ?$unbound)?)* >
            [ $head $($args)* ]
            ( $($leftover)* )
        );
        impl_fn_from_config!(
            $trait
            < $head: ?Sized $($letters $(: ?$unbound)?)* >
            [ & $head $($args)* ]
            ( $($leftover)* )
        );
        impl_fn_from_config!(
            $trait
            < $head: ?Sized $($letters $(: ?$unbound)?)* >
            [ &mut $head $($args)* ]
            ( $($leftover)* )
        );
    };
    (
        $trait:ident
        ( $head:ident $($leftover:ident)* )
    ) => {
        // First, branch out both the true implementation and the level below.
        impl_fn_from_config!(
            $trait
            <>
            []
            ( $head $($leftover)* )
        );
        impl_fn_from_config!(
            $trait
            ( $($leftover)* )
        );
    };
}

/// A wrapper around a value that cannot be parsed from config, but can still be stored/retrieved
/// in a context.
///
/// This brings a `Resolvable` implementation that will always fail.
pub struct NoConfig<T>(pub T);

impl<T> NoConfig<T> {
    /// Return the wrapped object.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for NoConfig<T> {
    fn from(t: T) -> Self {
        NoConfig(t)
    }
}

// Implement Resolvable for the wrapper, so we can resolve it.
impl<T> Resolvable for NoConfig<T> {
    // We leave from_config as default (it'll always fail).
    // As stated in the name, this cannot be loaded from a Config.

    // But when loading from a variable, accept an unwrapped value.
    //
    // So users can store a `T` and load it as `NoConfig<T>`.
    fn from_any(any: Box<dyn Any>) -> Option<Self>
    where
        Self: Sized + Any,
    {
        // First try an actual NoConfig<T>
        any.downcast()
            .map(|b| *b)
            // Then, try a bare T
            .or_else(|any| any.downcast::<T>().map(|b| NoConfig(*b)))
            .ok()
    }
}

// TODO: This could be solved with NoConfig instead.
// Implement Resolvable for all functions taking 4 or less arguments.
// (They will all fail to deserialize, but at least we can call resolve() on them)
// We could consider increasing that? It would probably increase compilation time, and clutter the
// Resolvable doc page. Maybe behind a feature if people really need it?
// (Ideally we wouldn't need it and we'd have a blanket implementation instead, but that may
// require specialization.)
impl_fn_from_config!(Resolvable (D C B A));

impl<T> Resolvable for Option<T>
where
    T: Resolvable,
{
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        if let Config::Null = config {
            Ok(None)
        } else {
            Ok(Some(T::from_config(config, context)?))
        }
    }

    fn from_any(any: Box<dyn Any>) -> Option<Self>
    where
        Self: Sized + Any,
    {
        // First try the option, then try bare T.
        any.downcast().map(|b| *b)
             // Here we have a Result<Option<T>, _>
            .unwrap_or_else(|any| T::from_any(any).map(|b| Some(b)))
    }
}

impl Resolvable for Config {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        Ok(config.clone())
    }
}

impl Resolvable for Object {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        config
            .as_object()
            .ok_or_else(|| Error::invalid_config("Expected an object", config))
            .cloned()
    }
}

impl Resolvable for Box<dyn crate::view::View> {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let boxed: BoxedView = context.build(config)?;
        Ok(boxed.unwrap())
    }
}

impl Resolvable for BoxedView {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        context.build(config)
    }
}

impl Resolvable for crate::theme::BaseColor {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        (|| Self::parse(config.as_str()?))().ok_or_else(|| Error::InvalidConfig {
            message: "Invalid config for BaseColor".into(),
            config: config.clone(),
        })
    }
}

impl Resolvable for crate::theme::Palette {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let mut palette = Self::default();

        let config = config
            .as_object()
            .ok_or_else(|| Error::invalid_config("Expected object", config))?;

        for (key, value) in config {
            if let Ok(value) = context.resolve(value) {
                palette.set_color(key, value);
            } else if let Some(value) = value.as_object() {
                // We don't currently support namespace themes here.
                // ¯\_(ツ)_/¯
                log::warn!(
                    "Namespaces are not currently supported in configs. (When reading color for `{key}`: {value:?}.)"
                );
            }
        }

        Ok(palette)
    }
}

impl Resolvable for crate::theme::BorderStyle {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let borders: String = context.resolve(config)?;

        Ok(Self::from(&borders))
    }
}

impl Resolvable for crate::theme::Theme {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let mut theme = Self::default();

        if let Some(shadow) = context.resolve(&config["shadow"])? {
            theme.shadow = shadow;
        }

        if let Some(borders) = context.resolve(&config["borders"])? {
            theme.borders = borders;
        }

        if let Some(palette) = context.resolve(&config["palette"])? {
            theme.palette = palette;
        }

        Ok(theme)
    }
}

// A bunch of `impl From<T: Resolvable>` can easily implement Resolvable
impl<T> Resolvable for Box<T>
where
    T: 'static + Resolvable,
{
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        Ok(Box::new(T::from_config(config, context)?))
    }

    fn from_any(any: Box<dyn Any>) -> Option<Self> {
        // First try a Box<T>
        match any.downcast::<Self>().map(|b| *b) {
            Ok(res) => Some(res),
            // If it fails, try T::from_any (unboxed stored value)
            Err(any) => T::from_any(any).map(Into::into),
        }
    }
}

impl<T> Resolvable for Arc<T>
where
    T: 'static + Resolvable,
{
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        Ok(Arc::new(T::from_config(config, context)?))
    }

    fn from_any(any: Box<dyn Any>) -> Option<Self> {
        // First try a Arc<T>
        match any.downcast::<Self>().map(|b| *b) {
            Ok(res) => Some(res),
            Err(any) => T::from_any(any).map(Into::into),
        }
    }
}

impl<T> Resolvable for HashMap<String, T>
where
    T: 'static + Resolvable,
{
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let config = match config {
            Config::Null => return Ok(HashMap::new()),
            Config::Object(config) => config,
            // Missing value get an empty vec
            _ => return Err(Error::invalid_config("Expected array", config)),
        };

        config
            .iter()
            .map(|(k, v)| context.resolve(v).map(|v| (k.to_string(), v)))
            .collect()
    }
}

impl<T> Resolvable for Vec<T>
where
    T: 'static + Resolvable,
{
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let config = match config {
            Config::Array(config) => config,
            // Missing value get an empty vec
            Config::Null => return Ok(Vec::new()),
            _ => return Err(Error::invalid_config("Expected array", config)),
        };

        config.iter().map(|v| context.resolve(v)).collect()
    }
}

impl<T, const N: usize> Resolvable for [T; N]
where
    T: 'static + Resolvable + Clone,
{
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let vec = Vec::<T>::from_config(config, context)?;
        vec.try_into()
            .map_err(|_| Error::invalid_config("Expected array of size {N}", config))
    }
}

// ```yaml
// color: red
// color:
//      dark: red
// color:
//      rgb: [1, 2, 4]
// ```
impl Resolvable for crate::theme::Color {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        Ok(match config {
            Config::String(config) => Self::parse(config)
                .ok_or_else(|| Error::invalid_config("Could not parse color", config))?,
            Config::Object(config) => {
                // Possibly keywords:
                // - light
                // - dark
                // - rgb
                let (key, value) = config
                    .iter()
                    .next()
                    .ok_or_else(|| Error::invalid_config("", config))?;
                match key.as_str() {
                    "light" => Self::Light(context.resolve(value)?),
                    "dark" => Self::Dark(context.resolve(value)?),
                    "rgb" => {
                        let array: [u8; 3] = context.resolve(value)?;
                        Self::Rgb(array[0], array[1], array[2])
                    }
                    _ => return Err(Error::invalid_config("Found unexpected key", config)),
                }
            }
            Config::Array(_) => {
                // Assume r, g, b
                let array: [u8; 3] = context.resolve(config)?;
                Self::Rgb(array[0], array[1], array[2])
            }
            _ => return Err(Error::invalid_config("Found unsupported type", config)),
        })
    }
}

impl Resolvable for crate::theme::PaletteColor {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let color: String = context.resolve(config)?;

        crate::theme::PaletteColor::from_str(&color)
            .map_err(|_| Error::invalid_config("Unrecognized palette color", config))
    }
}

impl Resolvable for crate::theme::ColorType {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        if let Ok(color) = context.resolve(config) {
            return Ok(Self::Color(color));
        }

        match config {
            Config::String(config) => Self::from_str(config)
                .map_err(|_| Error::invalid_config("Unrecognized color type", config)),
            Config::Object(config) => {
                // Try to load as a color?
                let (key, value) = config
                    .iter()
                    .next()
                    .ok_or_else(|| Error::invalid_config("Found empty object", config))?;
                Ok(match key.as_str() {
                    "palette" => Self::Palette(context.resolve(value)?),
                    "color" => Self::Color(context.resolve(value)?),
                    _ => {
                        return Err(Error::invalid_config(
                            format!("Found unrecognized key `{key}` in color type config"),
                            config,
                        ))
                    }
                })
            }
            _ => Err(Error::invalid_config("Expected string or object", config)),
        }
    }
}

impl Resolvable for crate::theme::ColorStyle {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        if let Ok(color) = (|| -> Result<_, Error> {
            let front = context.resolve(&config["front"])?;
            let back = context.resolve(&config["back"])?;

            Ok(crate::theme::ColorStyle { front, back })
        })() {
            return Ok(color);
        }

        unimplemented!()
    }
}

impl Resolvable for crate::view::Offset {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        if let Some("center" | "Center") = config.as_str() {
            return Ok(Self::Center);
        }

        let config = config
            .as_object()
            .ok_or_else(|| Error::invalid_config("Expected `center` or an object.", config))?;

        let (key, value) = config
            .iter()
            .next()
            .ok_or_else(|| Error::invalid_config("Expected non-empty object.", config))?;

        match key.as_str() {
            "Absolute" | "absolute" => Ok(Self::Absolute(context.resolve(value)?)),
            "Parent" | "parent" => Ok(Self::Parent(context.resolve(value)?)),
            _ => Err(Error::invalid_config("Unexpected key `{key}`.", config)),
        }
    }
}

// Literals don't need a context at all

impl Resolvable for String {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        match config.as_str() {
            Some(config) => Ok(config.into()),
            None => Err(Error::invalid_config("Expected string type", config)),
        }
    }
}

impl Resolvable for bool {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        config
            .as_bool()
            .ok_or_else(|| Error::invalid_config("Expected bool type", config))
    }
}

macro_rules! resolve_unsigned {
    ($ty:ty) => {
        impl Resolvable for $ty {
            fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
                config
                    .as_u64()
                    .and_then(|config| Self::try_from(config).ok())
                    .ok_or_else(|| {
                        Error::invalid_config(
                            format!("Expected unsigned <= {}", Self::max_value()),
                            config,
                        )
                    })
            }
        }
    };
}
macro_rules! resolve_signed {
    ($ty:ty) => {
        impl Resolvable for $ty {
            fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
                config
                    .as_i64()
                    .and_then(|config| Self::try_from(config).ok())
                    .ok_or_else(|| {
                        Error::invalid_config(
                            format!(
                                "Expected {} <= unsigned <= {}",
                                Self::min_value(),
                                Self::max_value()
                            ),
                            config,
                        )
                    })
            }
        }
    };
}

resolve_unsigned!(u8);
resolve_unsigned!(u16);
resolve_unsigned!(u32);
resolve_unsigned!(u64);
resolve_unsigned!(usize);

resolve_signed!(i8);
resolve_signed!(i16);
resolve_signed!(i32);
resolve_signed!(i64);
resolve_signed!(isize);

impl<T: Resolvable + 'static> Resolvable for crate::XY<T> {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        Ok(match config {
            Config::Array(config) if config.len() == 2 => {
                let x = context.resolve(&config[0])?;
                let y = context.resolve(&config[1])?;
                crate::XY::new(x, y)
            }
            Config::Object(config) => {
                let x = context.resolve(&config["x"])?;
                let y = context.resolve(&config["y"])?;
                crate::XY::new(x, y)
            }
            // That one would require specialization?
            // Config::String(config) if config == "zero" => crate::Vec2::zero(),
            config => {
                return Err(Error::invalid_config(
                    "Expected Array of length 2, object, or 'zero'.",
                    config,
                ))
            }
        })
    }
}

impl Resolvable for crate::direction::Orientation {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let value: String = context.resolve(config)?;
        Ok(match value.as_str() {
            "vertical" | "Vertical" => Self::Vertical,
            "horizontal" | "Horizontal" => Self::Horizontal,
            _ => {
                return Err(Error::invalid_config(
                    "Unrecognized orientation. Should be horizontal or vertical.",
                    config,
                ))
            }
        })
    }
}

impl Resolvable for crate::view::Margins {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        Ok(match config {
            Config::Object(config) => Self::lrtb(
                context.resolve(&config["left"])?,
                context.resolve(&config["right"])?,
                context.resolve(&config["top"])?,
                context.resolve(&config["bottom"])?,
            ),
            Config::Number(_) => {
                let n = context.resolve(config)?;
                Self::lrtb(n, n, n, n)
            }
            _ => return Err(Error::invalid_config("Expected object or number", config)),
        })
    }
}

impl Resolvable for crate::align::HAlign {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        // TODO: also resolve single-value configs like strings.
        // Also when resolving a variable with the wrong type, fallback on loading the type with
        // the variable name.
        Ok(match config.as_str() {
            Some(config) if config == "Left" || config == "left" => Self::Left,
            Some(config) if config == "Center" || config == "center" => Self::Center,
            Some(config) if config == "Right" || config == "right" => Self::Right,
            _ => {
                return Err(Error::invalid_config(
                    "Expected left, center or right",
                    config,
                ))
            }
        })
    }
}

new_default!(Context);

impl Context {
    /// Prepare a new context using registered recipes.
    pub fn new() -> Self {
        // Collect a distributed set of recipes.
        #[cfg(feature = "builder")]
        let recipes = inventory::iter::<Recipe>()
            .map(|recipe| recipe.as_tuple())
            .collect();

        #[cfg(not(feature = "builder"))]
        let recipes = Default::default();

        // for (recipe, _) in &recipes {
        //     eprintln!("{recipe:?}");
        // }

        #[cfg(feature = "builder")]
        let wrappers = inventory::iter::<WrapperRecipe>()
            .map(|recipe| recipe.as_tuple())
            .collect();

        #[cfg(not(feature = "builder"))]
        let wrappers = Default::default();

        // Store callback recipes as variables for now.
        #[cfg(feature = "builder")]
        let variables = inventory::iter::<CallbackRecipe>()
            .map(|recipe| recipe.as_tuple())
            .collect();

        #[cfg(not(feature = "builder"))]
        let variables = Default::default();

        let recipes = Arc::new(Recipes {
            recipes,
            wrappers,
            parent: None,
        });

        let variables = Arc::new(Variables {
            variables,
            parent: None,
        });

        Self { recipes, variables }
    }

    /// Resolve a value.
    ///
    /// Needs to be a reference to a variable.
    pub fn resolve_as_var<T: 'static + Resolvable>(&self, config: &Config) -> Result<T, Error> {
        // Use same strategy as for recipes: always include a "config", potentially null
        if let Some(name) = parse_var(config) {
            // log::info!("Trying to load variable {name:?}");
            // Option 1: a simple variable name.
            self.load(name, &Config::Null)
        } else if let Some(config) = config.as_object() {
            // Option 2: an object with a key (variable name pointing to a cb
            // recipe) and a body (config for the recipe).
            let (key, value) = config.iter().next().ok_or_else(|| Error::InvalidConfig {
                message: "Expected non-empty body".into(),
                config: config.clone().into(),
            })?;

            let key = key.strip_prefix('$').ok_or_else(|| Error::InvalidConfig {
                message: format!("Expected variable as $key; but found {key}."),
                config: config.clone().into(),
            })?;

            self.load(key, value)
        } else {
            // Here we did not find anything that looks like a variable.
            // Let's just bubble up, and hope we can use resolve() instead.
            Err(Error::CouldNotLoad {
                expected_type: std::any::type_name::<T>().into(),
                config: config.clone(),
            })
        }
    }

    fn resolve_as_builder<T: Resolvable + 'static>(&self, config: &Config) -> Result<T, Error> {
        // TODO: return a cheap error here, no allocation
        let config = config
            .as_object()
            .ok_or_else(|| Error::invalid_config("Expected string", config))?;

        // Option 2: an object with a key (variable name pointing to a cb
        // recipe) and a body (config for the recipe).
        let (key, value) = config.iter().next().ok_or_else(|| Error::InvalidConfig {
            message: "Expected non-empty body".into(),
            config: config.clone().into(),
        })?;

        let key = key.strip_prefix('$').ok_or_else(|| Error::InvalidConfig {
            message: "Expected variable as key".into(),
            config: config.clone().into(),
        })?;

        self.load(key, value)
    }

    /// Resolve a value straight from the config.
    ///
    /// This will not attempt to load the value as a variable if the config is a string.
    ///
    /// Note however that while loading as a config, it may still resolve nested values as
    /// variables.
    pub fn resolve_as_config<T: Resolvable + 'static>(&self, config: &Config) -> Result<T, Error> {
        // First, it could be a variable pointing to a config from a template view
        if let Some(name) = parse_var(config) {
            // Option 1: a simple variable name.
            if let Ok(var) = self.load(name, &Config::Null) {
                return Ok(var);
            }
        }

        if let Ok(value) = self.resolve_as_builder(config) {
            return Ok(value);
        }

        T::from_config(config, self)
    }

    /// Resolve a value
    pub fn resolve<T: Resolvable + 'static>(&self, config: &Config) -> Result<T, Error> {
        let var_failure = Box::new(match self.resolve_as_var(config) {
            Ok(value) => return Ok(value),
            Err(err) => err,
        });

        let config_failure = Box::new(match self.resolve_as_config(config) {
            Ok(value) => return Ok(value),
            Err(err) => err,
        });

        Err(Error::ResolveFailed {
            var_failure,
            config_failure,
        })
    }

    /// Resolve a value, using the given default if the key is missing.
    pub fn resolve_or<T: Resolvable + 'static>(
        &self,
        config: &Config,
        if_missing: T,
    ) -> Result<T, Error> {
        Ok(self.resolve::<Option<T>>(config)?.unwrap_or(if_missing))
    }

    fn store_entry(&mut self, name: impl Into<String>, entry: VarEntry) {
        // This will fail if there are any sub_context alive.
        // On the other hand, would anyone store variables after the fact?
        let name = name.into();
        if let Some(variables) = Arc::get_mut(&mut self.variables) {
            variables.store(name, entry);
        } else {
            log::error!("Context was not available to store variable `{name}`.");
        }
    }

    /// Store a new variable that can only be resolved once.
    pub fn store_once<T>(&mut self, name: impl Into<String>, value: T)
    where
        T: Send + 'static,
    {
        self.store_with(name, resolve_once(value));
    }

    /// Store a new variable maker.
    pub fn store_with<T: 'static>(
        &mut self,
        name: impl Into<String>,
        maker: impl 'static + Fn(&Config, &Context) -> Result<T, Error> + Send + Sync,
    ) {
        let name = name.into();
        // eprintln!(
        //     "Storing {name} with {:?} (for {})",
        //     std::any::TypeId::of::<T>(),
        //     std::any::type_name::<T>()
        // );
        let maker: AnyMaker = Box::new(move |config, context| {
            let res: T = (maker)(config, context)?;
            // eprintln!("Generated type ID: {:?}", res.type_id());
            let b: Box<dyn Any> = Box::new(res);
            // eprintln!("Boxed type ID: {:?}", b.as_ref().type_id());
            Ok(b)
        });

        self.store_entry(name, VarEntry::Maker(maker));
    }

    /// Store a new variable for resolution.
    ///
    /// Can be a callback, a usize, ...
    pub fn store<S, T: 'static>(&mut self, name: S, value: T)
    where
        S: Into<String>,
        T: Clone + Send + Sync,
    {
        self.store_with(name, move |_, _| Ok(value.clone()));
    }

    /// Store a view for resolution.
    ///
    /// The view can be resolved as a `BoxedView`.
    ///
    /// Note that this will only resolve this view _once_.
    ///
    /// If the view should be resolved more than that, consider calling `store_with` and
    /// re-constructing a `BoxedView` (maybe by cloning your view) every time.
    pub fn store_view<S, T>(&mut self, name: S, view: T)
    where
        S: Into<String>,
        T: crate::view::IntoBoxedView,
    {
        self.store_once(name, BoxedView::new(view.into_boxed_view()));
    }

    /// Store a new config.
    pub fn store_config(&mut self, name: impl Into<String>, config: impl Into<Config>) {
        self.store_entry(name, VarEntry::config(config));
    }

    /// Store a new variable proxy.
    pub fn store_proxy(&mut self, name: impl Into<String>, new_name: impl Into<String>) {
        self.store_entry(name, VarEntry::proxy(new_name));
    }

    /// Register a new recipe _for this context only_.
    pub fn register_recipe<F>(&mut self, name: impl Into<String>, recipe: F)
    where
        F: Fn(&Config, &Context) -> Result<BoxedView, Error> + 'static + Send + Sync,
    {
        if let Some(recipes) = Arc::get_mut(&mut self.recipes) {
            recipes.recipes.insert(name.into(), Box::new(recipe));
        }
    }

    /// Register a new wrapper recipe _for this context only_.
    pub fn register_wrapper_recipe<F>(&mut self, name: impl Into<String>, recipe: F)
    where
        F: Fn(&Config, &Context) -> Result<Wrapper, Error> + 'static + Send + Sync,
    {
        if let Some(recipes) = Arc::get_mut(&mut self.recipes) {
            recipes.wrappers.insert(name.into(), Box::new(recipe));
        }
    }

    /*
    /// Loads a variable of the given type.
    ///
    /// This does not require Resolvable on `T`, but will also not try to deserialize.
    ///
    /// It should be used for types that simply cannot be parsed from config, like closures.
    pub fn load_as_var<T: Any>(
        &self,
        name: &str,
        config: &Config,
    ) -> Result<T, Error> {
        self.variables.call_on_any(
            name,
            self.on_maker(name, config),
            |config| self.resolve_as_var(config),
        )
    }
    */

    // Helper function to implement load_as_var and load
    fn on_maker<T: 'static + Resolvable>(
        &self,
        maker: &AnyMaker,
        name: &str,
        config: &Config,
    ) -> Result<T, Error> {
        let res: Box<dyn Any> = (maker)(config, self)?;

        // eprintln!(
        //     "Trying to load {name} as {:?} (for {})",
        //     std::any::TypeId::of::<T>(),
        //     std::any::type_name::<T>()
        // );
        // eprintln!(
        //     "Loading var `{name}`: found type ID {:?}",
        //     res.as_ref().type_id()
        // );
        T::from_any(res).ok_or_else(|| {
            // It was not the right type :(
            Error::IncorrectVariableType {
                name: name.into(),
                expected_type: std::any::type_name::<T>().into(),
            }
        })
    }

    /// Loads a variable of the given type.
    ///
    /// If a variable with this name is found but is a Config, tries to deserialize it.
    ///
    /// Note: `config` can be `&Config::Null` for loading simple variables.
    pub fn load<T: Resolvable + Any>(&self, name: &str, config: &Config) -> Result<T, Error> {
        self.variables.call_on_any(
            name,
            |maker| self.on_maker(maker, name, config),
            |config| self.resolve(config),
        )
    }

    /// Build a wrapper with the given config
    pub fn build_wrapper(&self, config: &Config) -> Result<Wrapper, Error> {
        // Expect a single key
        let (key, value) = match config {
            Config::String(key) => (key, &Config::Null),
            Config::Object(config) => config.into_iter().next().ok_or(Error::InvalidConfig {
                message: "Expected non-empty object".into(),
                config: config.clone().into(),
            })?,
            _ => {
                return Err(Error::InvalidConfig {
                    message: "Expected string or object".into(),
                    config: config.clone(),
                })
            }
        };

        let wrapper = self.recipes.build_wrapper(key, value, self)?;

        Ok(wrapper)
    }

    /// Validate a config.
    ///
    /// Returns an error if any variable is missing or used more than once.
    pub fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        let mut vars = HashSet::new();

        let mut duplicates = HashSet::new();

        inspect_variables(config, &mut |variable| {
            if !vars.insert(variable.to_string()) {
                // Error! We found a duplicate!
                duplicates.insert(variable.to_string());
            }
        });

        let not_found: HashSet<String> = vars
            .into_iter()
            .filter(|var| !self.variables.variables.contains_key(var))
            .collect();

        ConfigError::from(duplicates, not_found)
    }

    fn get_wrappers(&self, config: &Config) -> Result<Vec<Wrapper>, Error> {
        fn get_with(config: &Config) -> Option<&Vec<Config>> {
            config.as_object()?.get("with")?.as_array()
        }

        let with = match get_with(config) {
            Some(with) => with,
            None => return Ok(Vec::new()),
        };

        with.iter().map(|with| self.build_wrapper(with)).collect()
    }

    /// Build a new view from the given config.
    pub fn build(&self, config: &Config) -> Result<BoxedView, Error> {
        let (key, value) = match config {
            // Some views can be built from a null config.
            Config::String(name) => (name, &serde_json::Value::Null),
            // Most view require a full object.
            Config::Object(config) => {
                // Expect a single key
                config.iter().next().ok_or(Error::InvalidConfig {
                    message: "Expected non-empty object".into(),
                    config: config.clone().into(),
                })?
            }
            _ => {
                return Err(Error::InvalidConfig {
                    message: "Expected object or string.".into(),
                    config: config.clone(),
                })
            }
        };

        let with = self.get_wrappers(value)?;

        let mut view = self.recipes.build(key, value, self)?;

        // Now, apply optional wrappers
        for wrapper in with {
            view = (wrapper)(view);
        }

        Ok(view)
    }

    /// Prepare a new context with some variable overrides.
    pub fn sub_context<F>(&self, f: F) -> Context
    where
        F: FnOnce(&mut Context),
    {
        let variables = Arc::new(Variables {
            variables: HashMap::new(),
            parent: Some(Arc::clone(&self.variables)),
        });

        let recipes = Arc::new(Recipes {
            recipes: HashMap::new(),
            wrappers: HashMap::new(),
            parent: Some(Arc::clone(&self.recipes)),
        });

        let mut context = Context { recipes, variables };
        f(&mut context);
        context
    }

    /// Builds a view from a template config.
    ///
    /// `template` should be a config describing a view, potentially using variables.
    /// Any value in `config` will be stored as a variable when rendering the template.
    pub fn build_template(&self, config: &Config, template: &Config) -> Result<BoxedView, Error> {
        let res = self
            .sub_context(|c| {
                if let Some(config) = config.as_object() {
                    for (key, value) in config.iter() {
                        // If value is a variable, resolve it first.
                        if let Some(var) = parse_var(value) {
                            c.store_proxy(key, var);
                        } else {
                            c.store_config(key, value.clone());
                        }
                    }
                } else {
                    c.store_config(".", config.clone());
                }
            })
            .build(template)?;

        Ok(res)
    }
}

fn parse_var(value: &Config) -> Option<&str> {
    value.as_str().and_then(|s| s.strip_prefix('$'))
}

impl Variables {
    /// Store a new variable for interpolation.
    ///
    /// Can be a callback, a usize, ...
    fn store<S>(&mut self, name: S, value: VarEntry)
    where
        S: Into<String>,
    {
        let name = name.into();
        // eprintln!(
        //     "Storing {name} with type {} (ID {:?})",
        //     std::any::type_name::<T>(),
        //     std::any::TypeId::of::<T>(),
        // );
        self.variables.insert(name, value);
    }

    fn call_on_any<OnMaker, OnConfig, T>(
        &self,
        name: &str,
        mut on_maker: OnMaker,
        mut on_config: OnConfig,
    ) -> Result<T, Error>
    where
        OnConfig: FnMut(&Config) -> Result<T, Error>,
        OnMaker: FnMut(&AnyMaker) -> Result<T, Error>,
        T: 'static,
    {
        let new_name = match self.variables.get(name) {
            None => None,
            Some(VarEntry::Proxy(proxy)) => Some(Arc::clone(proxy)),
            Some(VarEntry::Maker(maker)) => return (on_maker)(maker),
            Some(VarEntry::Config(config)) => return (on_config)(config),
        };

        let name = new_name
            .as_ref()
            .map(|s| s.as_ref().as_str())
            .unwrap_or(name);

        self.parent.as_ref().map_or_else(
            || Err(Error::NoSuchVariable(name.into())),
            |parent| parent.call_on_any(name, on_maker, on_config),
        )
    }
}

/// Describes how to build a callback.
pub struct CallbackRecipe {
    /// Name used in config file to use this callback.
    ///
    /// The config file will include an extra $ at the beginning.
    pub name: &'static str,

    /// Function to run this recipe.
    pub builder: BareVarBuilder,
}

impl CallbackRecipe {
    fn as_tuple(&self) -> (String, VarEntry) {
        let cb: AnyMaker = Box::new(self.builder);
        (self.name.into(), VarEntry::maker(cb))
    }
}

/// Describes how to build a view.
pub struct Recipe {
    /// Name used in config file to use this recipe.
    pub name: &'static str,

    /// Function to run this recipe.
    pub builder: BareBuilder,
}

impl Recipe {
    fn as_tuple(&self) -> (String, BoxedBuilder) {
        (self.name.into(), Box::new(self.builder))
    }
}

/// Describes how to build a view wrapper.
pub struct WrapperRecipe {
    /// Name used in config file to use this wrapper.
    pub name: &'static str,

    /// Function to run this recipe.
    pub builder: BareWrapperBuilder,
}

impl WrapperRecipe {
    fn as_tuple(&self) -> (String, BoxedWrapperBuilder) {
        (self.name.into(), Box::new(self.builder))
    }
}

#[cfg(feature = "builder")]
inventory::collect!(Recipe);
#[cfg(feature = "builder")]
inventory::collect!(CallbackRecipe);
#[cfg(feature = "builder")]
inventory::collect!(WrapperRecipe);

#[cfg(not(feature = "builder"))]
#[macro_export]
/// Define a recipe to build this view from a config file.
macro_rules! raw_recipe {
    ($name:ident from $config_builder:expr) => {};
    (with $name:ident, $builder:expr) => {};
    ($name:ident, $builder:expr) => {};
}
#[cfg(feature = "builder")]
#[macro_export]
/// Define a recipe to build this view from a config file.
macro_rules! raw_recipe {
    ($name:ident from $config_builder:expr) => {
        $crate::submit! {
            $crate::builder::Recipe {
                name: stringify!($name),
                builder: |config, context| {
                    let template = $config_builder;
                    context.build_template(config, &template)
                },
            }
        }
    };
    (with $name:ident, $builder:expr) => {
        $crate::submit! {
            $crate::builder::WrapperRecipe {
                name: stringify!($name),
                builder: |config, context| {
                    let builder: fn(&$crate::reexports::serde_json::Value, &$crate::builder::Context) -> Result<_, $crate::builder::Error> = $builder;
                    let wrapper = (builder)(config, context)?;

                    Ok(Box::new(move |view| {
                        let view = (wrapper)(view);
                        $crate::views::BoxedView::boxed(view)
                    }))
                }
            }
        }
    };
    ($name:ident, $builder:expr) => {
        $crate::submit! {
            $crate::builder::Recipe {
                name: stringify!($name),
                builder: |config, context| {
                    let builder: fn(&$crate::reexports::serde_json::Value, &$crate::builder::Context) -> Result<_,$crate::builder::Error> = $builder;
                    (builder)(config, context).map($crate::views::BoxedView::boxed)
                },
            }
        }
    };
}

#[cfg(not(feature = "builder"))]
#[macro_export]
/// Define a macro for a variable builder.
macro_rules! var_recipe {
    ($name: expr, $builder:expr) => {};
}

#[cfg(feature = "builder")]
#[macro_export]
/// Define a macro for a variable builder.
macro_rules! var_recipe {
    ($name: expr, $builder:expr) => {
        $crate::submit! {
            $crate::builder::CallbackRecipe {
                name: $name,
                builder: |config, context| {
                    let builder: fn(&::serde_json::Value, &$crate::builder::Context) -> Result<_, $crate::builder::Error> = $builder;
                    Ok(Box::new((builder)(config, context)?))
                },
            }
        }
    };
}

// Simple recipe allowing to use variables as views, and attach a `with` clause.
raw_recipe!(View, |config, context| {
    let view: BoxedView = context.resolve(&config["view"])?;
    Ok(view)
});

// TODO: A $format recipe that parses a f-string and renders variables in there.
// Will need to look for various "string-able" types as variables.
// (String mostly, maybe integers)
// Probably needs regex crate to parse the template.

var_recipe!("concat", |config, context| {
    let values = config
        .as_array()
        .ok_or_else(|| Error::invalid_config("Expected array", config))?;

    values
        .iter()
        .map(|value| {
            // log::info!("Resolving {value:?}");
            context.resolve::<String>(value)
        })
        .collect::<Result<String, _>>()
});

#[cfg(feature = "builder")]
#[cfg(test)]
mod tests {

    #[test]
    fn test_load_config() {
        use crate::view::Finder;

        let config = r#"
            LinearLayout:
                children:
                    - TextView:
                        content: $foo
                        with:
                            - name: text
                    - DummyView
                    - TextView: bar
                    - LinearLayout:
                        orientation: horizontal
                        children:
                            - TextView: "Age?"
                            - DummyView
                            - EditView:
                                with:
                                    - name: edit
                with:
                    - full_screen
        "#;

        let foo = "Foo";

        let config: crate::builder::Config = serde_yaml::from_str(config).unwrap();

        let mut context = crate::builder::Context::new();

        // Here we're still missing the $foo variable.
        assert!(context.validate(&config).is_err());

        context.store("foo", foo.to_string());

        // Now everything is find.
        context.validate(&config).unwrap();

        // Build the view from the config
        let mut res = context.build(&config).unwrap();

        // The top-level view should be a full-screen view
        assert!(res
            .downcast_ref::<crate::views::ResizedView<crate::views::BoxedView>>()
            .is_some());

        // The view should be reachable by name
        let content = res
            .call_on_name("text", |v: &mut crate::views::TextView| v.get_content())
            .unwrap();

        assert_eq!(content.source(), foo);
    }
}
