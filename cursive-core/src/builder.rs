//! Build views from configuration.
//!
//! # Features
//!
//! This module is only active if the `builder` feature is enabled. Otherwise, types will still be
//! exposed, blueprints can be defined, but they will be ignored.
//!
//! # Overview
//!
//! This module lets you build a view from a json-like config object.
//!
//! For example, this yaml could be parsed and used to build a basic TextView:
//!
//! ```yaml
//! TextView:
//!     content: foo
//! ```
//!
//! Or, this slightly larger example could build a LinearLayout, relying on a `left_label`
//! variable that would need to be fed:
//!
//! ```yaml
//! LinearLayout:
//!     orientation: horizontal
//!     children:
//!         - TextView: $left_label
//!         - TextView: Center
//!         - Button:
//!             label: Right
//!             callback: $Cursive.quit
//! ```
//!
//! ## Configs
//!
//! Views are described using a `Config`, which is really just an alias for a `serde_json::Value`.
//! Note that we use the json model here, but you could parse it from JSON, yaml, or almost any
//! language supported by serde.
//!
//! ## Context
//!
//! A `Context` helps building views by providing variables that can be used by configs. They also
//! keep a list of available blueprints.
//!
//! ## Blueprints
//!
//! At the core of the builder system, blueprints define _how_ to build views.
//!
//! A blueprint essentially ties a name to a function `fn(Config, Context) -> Result<impl View>`.
//!
//! They are defined using macros - either manually (`manual_blueprint!`) or declaratively
//! (`#[blueprint]`). When a `Context` is created, they are automatically gathered from all
//! dependencies - so third party crates can define blueprints too.
//!
//! ## Resolving things
//!
//! Blueprints will need to parse various types from the config to build their views - strings,
//! integers, callbacks, ...
//!
//! To do this, they will rely on the `Resolvable` trait.
//!
//! # Examples
//!
//! You can see the [`builder` example][builder.rs] and its [yaml config][config].
//!
//! [builder.rs]: https://github.com/gyscos/cursive/blob/main/cursive/examples/builder.rs
//! [config]: https://github.com/gyscos/cursive/blob/main/cursive/examples/builder.yaml
#![cfg_attr(not(feature = "builder"), allow(unused))]

mod resolvable;

pub use self::resolvable::{NoConfig, Resolvable};

use crate::views::BoxedView;

use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
pub type Wrapper = Box<dyn FnOnce(BoxedView) -> BoxedView + Send>;

/// Can build a callback
pub type BareVarBuilder = fn(&serde_json::Value, &Context) -> Result<Box<dyn Any>, Error>;

/// Boxed variable builder
///
/// If you store a variable of this type, when loading type `T`, it will run
/// this builder and try to downcast the result to `T`.
pub type BoxedVarBuilder =
    Arc<dyn Fn(&serde_json::Value, &Context) -> Result<Box<dyn Any>, Error> + Send + Sync>;

/// Everything needed to prepare a view from a config.
///
/// - Current blueprints
/// - Any stored variables/callbacks
///
/// Cheap to clone (uses `Arc` internally).
#[derive(Clone)]
pub struct Context {
    // TODO: Merge variables and blueprints?
    // TODO: Use RefCell? Or even Arc<Mutex>?
    // So we can still modify the context when sub-context are alive.
    variables: Arc<Variables>,
    blueprints: Arc<Blueprints>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vars: Vec<_> = self.variables.keys().collect();
        let blueprints: Vec<_> = self.blueprints.keys().collect();
        let wrappers: Vec<_> = self.blueprints.wrapper_keys().collect();

        write!(f, "Variables: {vars:?}, ")?;
        write!(f, "Blueprints: {blueprints:?}, ")?;
        write!(f, "Wrappers: {wrappers:?}")?;

        Ok(())
    }
}

struct Blueprints {
    blueprints: HashMap<String, BoxedBuilder>,
    wrappers: HashMap<String, BoxedWrapperBuilder>,
    parent: Option<Arc<Blueprints>>,
}

/// Wrapper around a value that makes it Cloneable, but can only be resolved once.
pub struct ResolveOnce<T>(Arc<Mutex<Option<T>>>);

/// Return a variable-maker (for use in store_with)
pub fn resolve_once<T>(value: T) -> impl Fn(&Config, &Context) -> Result<T, Error>
where
    T: Send,
{
    let value = Mutex::new(Some(value));
    move |_, _| {
        value
            .lock()
            .take()
            .ok_or_else(|| Error::MakerFailed("variable was already resolved".to_string()))
    }
}

impl<T> ResolveOnce<T> {
    /// Create a new `ResolveOnce` which will resolve, once, to the given value.
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(Some(value))))
    }

    /// Take the value from self.
    pub fn take(&self) -> Option<T> {
        self.0.lock().take()
    }

    /// Check if there is a value still to be resolved in self.
    pub fn is_some(&self) -> bool {
        self.0.lock().is_some()
    }
}

impl Blueprints {
    fn wrapper_keys(&self) -> impl Iterator<Item = &String> {
        self.wrappers
            .keys()
            .chain(self.parent.iter().flat_map(|parent| {
                let parent: Box<dyn Iterator<Item = &String>> = Box::new(parent.wrapper_keys());
                parent
            }))
    }

    fn keys(&self) -> impl Iterator<Item = &String> {
        self.blueprints
            .keys()
            .chain(self.parent.iter().flat_map(|parent| {
                let parent: Box<dyn Iterator<Item = &String>> = Box::new(parent.keys());
                parent
            }))
    }

    fn build(&self, name: &str, config: &Config, context: &Context) -> Result<BoxedView, Error> {
        if let Some(blueprint) = self.blueprints.get(name) {
            (blueprint)(config, context)
                .map_err(|e| Error::BlueprintFailed(name.into(), Box::new(e)))
        } else {
            match self.parent {
                Some(ref parent) => parent.build(name, config, context),
                None => Err(Error::BlueprintNotFound(name.into())),
            }
        }
    }

    fn build_wrapper(
        &self,
        name: &str,
        config: &Config,
        context: &Context,
    ) -> Result<Wrapper, Error> {
        if let Some(blueprint) = self.wrappers.get(name) {
            (blueprint)(config, context)
                .map_err(|e| Error::BlueprintFailed(name.into(), Box::new(e)))
        } else {
            match self.parent {
                Some(ref parent) => parent.build_wrapper(name, config, context),
                None => Err(Error::BlueprintNotFound(name.into())),
            }
        }
    }
}

enum VarEntry {
    // Proxy variable used for sub-templates
    Proxy(Arc<String>),

    // Regular variable set by user or blueprint
    //
    // Set by user:
    //  - Clone-able
    //  -
    Maker(AnyMaker),

    // Embedded config by intermediate node
    Config(Config),
    // Optional: store blueprints separately?
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

    /// All variants from a multi-variant blueprint failed.
    AllVariantsFailed {
        /// Config value that could not be parsed
        config: Config,
        /// List of errors for the blueprint variants.
        errors: Vec<Error>,
    },

    /// A blueprint was not found
    BlueprintNotFound(String),

    /// A blueprint failed to run.
    ///
    /// This is in direct cause to an error in an actual blueprint.
    BlueprintFailed(String, Box<Error>),

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
new_default!(Context);

impl Context {
    /// Prepare a new context using registered blueprints.
    pub fn new() -> Self {
        // Collect a distributed set of blueprints.
        #[cfg(feature = "builder")]
        let blueprints = inventory::iter::<Blueprint>()
            .map(|blueprint| blueprint.as_tuple())
            .collect();

        #[cfg(not(feature = "builder"))]
        let blueprints = Default::default();

        // for (blueprint, _) in &blueprints {
        //     eprintln!("{blueprint:?}");
        // }

        #[cfg(feature = "builder")]
        let wrappers = inventory::iter::<WrapperBlueprint>()
            .map(|blueprint| blueprint.as_tuple())
            .collect();

        #[cfg(not(feature = "builder"))]
        let wrappers = Default::default();

        // Store callback blueprints as variables for now.
        #[cfg(feature = "builder")]
        let variables = inventory::iter::<CallbackBlueprint>()
            .map(|blueprint| blueprint.as_tuple())
            .collect();

        #[cfg(not(feature = "builder"))]
        let variables = Default::default();

        let blueprints = Arc::new(Blueprints {
            blueprints,
            wrappers,
            parent: None,
        });

        let variables = Arc::new(Variables {
            variables,
            parent: None,
        });

        Self {
            blueprints,
            variables,
        }
    }

    /// Resolve a value.
    ///
    /// Needs to be a reference to a variable.
    pub fn resolve_as_var<T: 'static + Resolvable>(&self, config: &Config) -> Result<T, Error> {
        // Use same strategy as for blueprints: always include a "config", potentially null
        if let Some(name) = parse_var(config) {
            // log::info!("Trying to load variable {name:?}");
            // Option 1: a simple variable name.
            self.load(name, &Config::Null)
        } else if let Some(config) = config.as_object() {
            // Option 2: an object with a key (variable name pointing to a cb
            // blueprint) and a body (config for the blueprint).
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
        // blueprint) and a body (config for the blueprint).
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
    pub fn store<S, T>(&mut self, name: S, value: T)
    where
        S: Into<String>,
        T: Clone + Send + Sync + 'static,
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

    /// Register a new blueprint _for this context only_.
    pub fn register_blueprint<F>(&mut self, name: impl Into<String>, blueprint: F)
    where
        F: Fn(&Config, &Context) -> Result<BoxedView, Error> + 'static + Send + Sync,
    {
        if let Some(blueprints) = Arc::get_mut(&mut self.blueprints) {
            blueprints
                .blueprints
                .insert(name.into(), Box::new(blueprint));
        }
    }

    /// Register a new wrapper blueprint _for this context only_.
    pub fn register_wrapper_blueprint<F>(&mut self, name: impl Into<String>, blueprint: F)
    where
        F: Fn(&Config, &Context) -> Result<Wrapper, Error> + 'static + Send + Sync,
    {
        if let Some(blueprints) = Arc::get_mut(&mut self.blueprints) {
            blueprints.wrappers.insert(name.into(), Box::new(blueprint));
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

        let wrapper = self.blueprints.build_wrapper(key, value, self)?;

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

        let mut view = self.blueprints.build(key, value, self)?;

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

        let blueprints = Arc::new(Blueprints {
            blueprints: HashMap::new(),
            wrappers: HashMap::new(),
            parent: Some(Arc::clone(&self.blueprints)),
        });

        let mut context = Context {
            blueprints,
            variables,
        };
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
    fn keys(&self) -> impl Iterator<Item = &String> {
        self.variables
            .keys()
            .chain(self.parent.iter().flat_map(|parent| {
                let parent: Box<dyn Iterator<Item = &String>> = Box::new(parent.keys());
                parent
            }))
    }

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
pub struct CallbackBlueprint {
    /// Name used in config file to use this callback.
    ///
    /// The config file will include an extra $ at the beginning.
    pub name: &'static str,

    /// Function to run this blueprint.
    pub builder: BareVarBuilder,
}

impl CallbackBlueprint {
    fn as_tuple(&self) -> (String, VarEntry) {
        let cb: AnyMaker = Box::new(self.builder);
        (self.name.into(), VarEntry::maker(cb))
    }
}

/// Describes how to build a view.
pub struct Blueprint {
    /// Name used in config file to use this blueprint.
    pub name: &'static str,

    /// Function to run this blueprint.
    pub builder: BareBuilder,
}

impl Blueprint {
    fn as_tuple(&self) -> (String, BoxedBuilder) {
        (self.name.into(), Box::new(self.builder))
    }
}

/// Describes how to build a view wrapper.
pub struct WrapperBlueprint {
    /// Name used in config file to use this wrapper.
    pub name: &'static str,

    /// Function to run this blueprint.
    pub builder: BareWrapperBuilder,
}

impl WrapperBlueprint {
    fn as_tuple(&self) -> (String, BoxedWrapperBuilder) {
        (self.name.into(), Box::new(self.builder))
    }
}

#[cfg(feature = "builder")]
inventory::collect!(Blueprint);
#[cfg(feature = "builder")]
inventory::collect!(CallbackBlueprint);
#[cfg(feature = "builder")]
inventory::collect!(WrapperBlueprint);

#[cfg(not(feature = "builder"))]
#[macro_export]
/// Define a blueprint to build this view from a config file.
macro_rules! manual_blueprint {
    ($name:ident from $config_builder:expr) => {};
    (with $name:ident, $builder:expr) => {};
    ($name:ident, $builder:expr) => {};
}

#[cfg(feature = "builder")]
#[macro_export]
/// Define a blueprint to manually build this view from a config file.
///
/// Note: this is entirely ignored (not even type-checked) if the `builder` feature is not
/// enabled.
///
/// There are 3 variants of this macro:
///
/// * `manual_blueprint!(Identifier, |config, context| make_the_view(...))`
///   This registers the recipe under `Identifier`, and uses the given closure to build
///   the view.
/// * `manual_blueprint!(Identifier from { parse_some_config(...) })`
///   This register under `Identifier` a recipe that forwards the creation to another
///   config using [`Context::build_template`].
/// * `manual_blueprint`(with Identifier, |config, context| Ok(|view| wrap_the_view(view, ...)))`
///   This register a "with" blueprint under `Identifier`, which will prepare a view wrapper.
macro_rules! manual_blueprint {
    // Remember to keep the inactive version above in sync
    ($name:ident from $config_builder:expr) => {
        $crate::submit! {
            $crate::builder::Blueprint {
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
            $crate::builder::WrapperBlueprint {
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
            $crate::builder::Blueprint {
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
macro_rules! fn_blueprint {
    ($name: expr, $builder:expr) => {};
}

#[cfg(feature = "builder")]
#[macro_export]
/// Define a macro for a variable builder.
macro_rules! fn_blueprint {
    // Remember to keep the inactive version above in sync
    ($name: expr, $builder:expr) => {
        $crate::submit! {
            $crate::builder::CallbackBlueprint {
                name: $name,
                builder: |config, context| {
                    let builder: fn(&::serde_json::Value, &$crate::builder::Context) -> Result<_, $crate::builder::Error> = $builder;
                    Ok(Box::new((builder)(config, context)?))
                },
            }
        }
    };
}

// Simple blueprint allowing to use variables as views, and attach a `with` clause.
manual_blueprint!(View, |config, context| {
    let view: BoxedView = context.resolve(&config["view"])?;
    Ok(view)
});

// TODO: A $format blueprint that parses a f-string and renders variables in there.
// Will need to look for various "string-able" types as variables.
// (String mostly, maybe integers)
// Probably needs regex crate to parse the template.

fn_blueprint!("concat", |config, context| {
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

fn_blueprint!("cursup", |config, context| {
    let text: String = context.resolve(config)?;

    Ok(crate::utils::markup::cursup::parse(text))
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
