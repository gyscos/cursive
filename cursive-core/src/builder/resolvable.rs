use super::{Config, Context, Error, Object};
use crate::views::BoxedView;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use std::any::Any;

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
#[derive(Clone, Debug, Eq, PartialEq)]
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
    // So users can store a `T` and load it as `NoConfig<T>`, without having to implement
    // `Resolvable` for `T`.
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
        any.downcast::<Self>()
            .map(|b| *b)
            .or_else(|any| T::from_any(any).map(|b| Some(b)).ok_or(()))
            .ok()
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

impl Resolvable for crate::style::BaseColor {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        (|| Self::parse(config.as_str()?))().ok_or_else(|| Error::InvalidConfig {
            message: "Invalid config for BaseColor".into(),
            config: config.clone(),
        })
    }
}

impl Resolvable for crate::style::Palette {
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

impl Resolvable for crate::style::BorderStyle {
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
            _ => return Err(Error::invalid_config("Expected object", config)),
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

    // TODO: Allow loading from `Vec<Box<Any>>` and downcasting one by one?
}

impl<T, const N: usize> Resolvable for [T; N]
where
    T: 'static + Resolvable + Clone,
{
    fn from_any(any: Box<dyn Any>) -> Option<Self>
    where
        Self: Sized + Any,
    {
        // Allow storing a `Vec` with the correct size
        any.downcast()
            .map(|b| *b)
            .or_else(|any| {
                any.downcast::<Vec<T>>()
                    .ok()
                    .and_then(|vec| (*vec).try_into().ok())
                    .ok_or(())
            })
            .ok()
    }

    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let vec = Vec::<T>::from_config(config, context)?;
        vec.try_into()
            .map_err(|_| Error::invalid_config("Expected array of size {N}", config))
    }
}

impl Resolvable for crate::style::Rgb<f32> {
    fn from_any(any: Box<dyn Any>) -> Option<Self> {
        // Accept both Rgb<u8> and Rgb<f32> as stored values.
        any.downcast()
            .map(|b| *b)
            .or_else(|any| {
                any.downcast::<crate::style::Rgb<u8>>()
                    .map(|rgb| rgb.as_f32())
            })
            .ok()
    }

    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        // Try as a hex string?
        if let Ok(rgb) = context.resolve::<String>(config) {
            if let Ok(rgb) = rgb.parse::<crate::style::Rgb<u8>>() {
                return Ok(rgb.as_f32());
            }
        }

        // Allow storing a list of f32 or a list of u8.
        if let Ok(rgb) = context.resolve::<[u8; 3]>(config) {
            // Try u8 first. If it's all u8, trying as f32 would also work.
            return Ok(crate::style::Rgb::<u8>::from(rgb).as_f32());
        }

        if let Ok(rgb) = context.resolve::<[f32; 3]>(config) {
            return Ok(Self::from(rgb));
        }

        // TODO: Here too, try as u8 first, then again as f32.
        if let Some(rgb) = config.as_object().and_then(|config| {
            let r = config.get("r").or_else(|| config.get("R"))?;
            let g = config.get("g").or_else(|| config.get("G"))?;
            let b = config.get("b").or_else(|| config.get("B"))?;

            let r = context.resolve(r).ok()?;
            let g = context.resolve(g).ok()?;
            let b = context.resolve(b).ok()?;

            Some(Self { r, g, b })
        }) {
            return Ok(rgb);
        }

        Err(Error::invalid_config(
            "Could not parse as a RGB color.",
            config,
        ))
    }
}

impl Resolvable for crate::style::Rgb<u8> {
    fn from_any(any: Box<dyn Any>) -> Option<Self>
    where
        Self: Sized + Any,
    {
        // Accept both Rgb<u8> and Rgb<f32> as stored values.
        any.downcast()
            .map(|b| *b)
            .or_else(|any| {
                any.downcast::<crate::style::Rgb<f32>>()
                    .map(|rgb| rgb.as_u8())
            })
            .ok()
    }

    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        // Try as a hex string?
        if let Ok(rgb) = context.resolve::<String>(config) {
            if let Ok(rgb) = rgb.parse::<crate::style::Rgb<u8>>() {
                return Ok(rgb);
            }
        }

        // Allow storing a list of f32 or a list of u8.
        if let Ok(rgb) = context.resolve::<[u8; 3]>(config) {
            // Try u8 first. If it's all u8, trying as f32 would also work.
            return Ok(Self::from(rgb));
        }

        // TODO: Here too, try as u8 first, then again as f32.
        if let Some(rgb) = config.as_object().and_then(|config| {
            let r = config.get("r").or_else(|| config.get("R"))?;
            let g = config.get("g").or_else(|| config.get("G"))?;
            let b = config.get("b").or_else(|| config.get("B"))?;

            let r = context.resolve(r).ok()?;
            let g = context.resolve(g).ok()?;
            let b = context.resolve(b).ok()?;

            Some(Self { r, g, b })
        }) {
            return Ok(rgb);
        }

        let rgb: [f32; 3] = context.resolve(config)?;
        Ok(crate::style::Rgb::<f32>::from(rgb).as_u8())
    }
}

impl Resolvable for crate::style::gradient::Dynterpolator {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let config = config
            .as_object()
            .ok_or_else(|| Error::invalid_config("Expected object", config))?;
        match config
            .iter()
            .next()
            .map(|(key, config)| (key.as_str(), config))
            .ok_or_else(|| Error::invalid_config("Expected non-empty object", config))?
        {
            ("radial", config) => {
                let radial: crate::style::gradient::Radial = context.resolve(config)?;
                Ok(Box::new(radial))
            }
            ("angled", config) => {
                let angled: crate::style::gradient::Angled = context.resolve(config)?;
                Ok(Box::new(angled))
            }
            ("bilinear", config) => {
                let bilinear: crate::style::gradient::Bilinear = context.resolve(config)?;
                Ok(Box::new(bilinear))
            }
            // TODO: Allow external libraries to define their own blueprints to be used here?
            // Something like a type-map of blueprints?...
            (key, _) => Err(Error::invalid_config(
                format!("Received unsupported gradient type {key}."),
                config,
            )),
        }
    }
}

impl Resolvable for crate::style::gradient::Bilinear {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let top_left = context.resolve(&config["top_left"])?;
        let top_right = context.resolve(&config["top_right"])?;
        let bottom_right = context.resolve(&config["bottom_right"])?;
        let bottom_left = context.resolve(&config["bottom_left"])?;
        Ok(Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        })
    }
}

impl Resolvable for crate::style::gradient::Angled {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let angle_rad = match context.resolve(&config["angle_rad"]) {
            Ok(angle_rad) => angle_rad,
            Err(err1) => match context.resolve::<f32>(&config["angle_deg"]) {
                Ok(angle_deg) => angle_deg * std::f32::consts::PI / 180f32,
                Err(err2) => {
                    return Err(Error::AllVariantsFailed {
                        config: config.clone(),
                        errors: vec![err1, err2],
                    })
                }
            },
        };
        let gradient = context.resolve(&config["gradient"])?;
        Ok(Self {
            angle_rad,
            gradient,
        })
    }
}

impl Resolvable for crate::style::gradient::Radial {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let center = context.resolve(&config["center"])?;
        let gradient = context.resolve(&config["gradient"])?;
        Ok(Self { center, gradient })
    }
}

impl Resolvable for crate::style::gradient::Linear {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        use crate::style::Rgb;

        // Options:
        // - A list of Rgb (evenly spaced)
        // - A list of (f32, Rgb)
        // - An object with start, end, and optionally middle, with a list of (f32, Rgb)
        // - Some presets strings? Rainbow?
        match config {
            Config::Array(array) => {
                let mut errors = Vec::new();

                match array
                    .iter()
                    .map(|config| context.resolve::<Rgb<f32>>(config))
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(colors) => return Ok(Self::evenly_spaced(&colors)),
                    Err(err) => errors.push(err),
                }

                match array
                    .iter()
                    .map(|config| context.resolve(config))
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(points) => return Ok(Self::new(points)),
                    Err(err) => errors.push(err),
                }

                return Err(Error::AllVariantsFailed {
                    config: config.clone(),
                    errors,
                });
            }
            Config::Object(object) => {
                if let (Some(start), Some(end)) = (object.get("start"), object.get("end")) {
                    return Ok(Self::simple(
                        context.resolve::<Rgb<f32>>(start)?,
                        context.resolve::<Rgb<f32>>(end)?,
                    ));
                }

                if let Some(points) = object.get("points") {
                    let points = points
                        .as_array()
                        .ok_or_else(|| Error::invalid_config("Expected array", config))?;

                    let points = points
                        .iter()
                        .map(|config| context.resolve(config))
                        .collect::<Result<Vec<_>, _>>()?;

                    return Ok(Self::new(points));
                }
            }
            Config::String(string) => match string.as_str() {
                // TODO: Allow external libs to define their own aliases to resolve here?
                "rainbow" => return Ok(Self::rainbow()),
                "black_to_white" | "black to white" => return Ok(Self::black_to_white()),
                _ => (),
            },
            _ => (),
        }

        Err(Error::invalid_config(
            "Expected array, object or string",
            config,
        ))
    }
}

// ```yaml
// color: red
// color:
//      dark: red
// color:
//      rgb: [1, 2, 4]
// ```
impl Resolvable for crate::style::Color {
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

impl Resolvable for crate::style::PaletteColor {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        let color: String = context.resolve(config)?;

        crate::style::PaletteColor::from_str(&color)
            .map_err(|_| Error::invalid_config("Unrecognized palette color", config))
    }
}

impl Resolvable for crate::style::ColorType {
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

impl Resolvable for crate::style::ColorStyle {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error> {
        if let Ok(color) = (|| -> Result<_, Error> {
            let front = context.resolve(&config["front"])?;
            let back = context.resolve(&config["back"])?;

            Ok(crate::style::ColorStyle { front, back })
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

impl Resolvable for String {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        match config.as_str() {
            Some(config) => Ok(config.into()),
            None => Err(Error::invalid_config("Expected string type", config)),
        }
    }
}

impl<A, B> Resolvable for (A, B)
where
    A: Resolvable + 'static,
    B: Resolvable + 'static,
{
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let config = config
            .as_array()
            .ok_or_else(|| Error::invalid_config("Expected array", config))?;

        Ok((context.resolve(&config[0])?, context.resolve(&config[1])?))
    }
}

impl<A, B, C> Resolvable for (A, B, C)
where
    A: Resolvable + 'static,
    B: Resolvable + 'static,
    C: Resolvable + 'static,
{
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let config = config
            .as_array()
            .ok_or_else(|| Error::invalid_config("Expected array", config))?;

        Ok((
            context.resolve(&config[0])?,
            context.resolve(&config[1])?,
            context.resolve(&config[2])?,
        ))
    }
}

impl Resolvable for crate::utils::markup::StyledString {
    fn from_config(config: &Config, context: &Context) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let text: String = context.resolve(config)?;
        Ok(Self::plain(text))
    }

    fn from_any(any: Box<dyn Any>) -> Option<Self>
    where
        Self: Sized + Any,
    {
        let any = match any.downcast::<Self>().map(|b| *b) {
            Ok(res) => return Some(res),
            Err(any) => any,
        };

        any.downcast::<String>().map(|b| Self::plain(*b)).ok()
    }
}

impl Resolvable for bool {
    fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
        config
            .as_bool()
            .ok_or_else(|| Error::invalid_config("Expected bool type", config))
    }
}

macro_rules! resolve_float {
    ($ty:ty) => {
        impl Resolvable for $ty {
            fn from_any(any: Box<dyn Any>) -> Option<Self>
            {
                // Accept both f32 and f64, just cast between them.
                any.downcast::<f32>()
                    .map(|b| *b as Self)
                    .or_else(|any| {
                        any.downcast::<f64>()
                            .map(|b| *b as Self)
                    })
                    .ok()
            }

            fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
                config
                    .as_f64()  // This already handles converting from integers
                    .map(|config| config as Self)
                    .ok_or_else(|| Error::invalid_config(format!("Expected float value"), config))
            }
        }
    };
}

macro_rules! resolve_unsigned {
    ($ty:ty) => {
        impl Resolvable for $ty {
            fn from_any(any: Box<dyn Any>) -> Option<Self> {
                // Accept any signed or unsigned integer, as long as it fits.
                any.downcast::<u8>()
                    .map(|b| (*b).try_into().ok())
                    .or_else(|any| any.downcast::<u16>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<u32>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<u64>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<u64>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<u128>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<usize>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i8>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i16>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i32>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i64>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i128>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<isize>().map(|b| (*b).try_into().ok()))
                    .ok()?
            }

            fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
                config
                    .as_u64()
                    .and_then(|config| Self::try_from(config).ok())
                    .ok_or_else(|| {
                        Error::invalid_config(format!("Expected unsigned <= {}", Self::MAX), config)
                    })
            }
        }
    };
}

macro_rules! resolve_signed {
    ($ty:ty) => {
        impl Resolvable for $ty {
            fn from_any(any: Box<dyn Any>) -> Option<Self> {
                // Accept any signed or unsigned integer, as long as it fits.
                any.downcast::<u8>()
                    .map(|b| (*b).try_into().ok())
                    .or_else(|any| any.downcast::<u16>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<u32>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<u64>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<u64>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<u128>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<usize>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i8>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i16>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i32>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i64>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<i128>().map(|b| (*b).try_into().ok()))
                    .or_else(|any| any.downcast::<isize>().map(|b| (*b).try_into().ok()))
                    .ok()?
            }

            fn from_config(config: &Config, _context: &Context) -> Result<Self, Error> {
                config
                    .as_i64()
                    .and_then(|config| Self::try_from(config).ok())
                    .ok_or_else(|| {
                        Error::invalid_config(
                            format!("Expected {} <= unsigned <= {}", Self::MIN, Self::MAX,),
                            config,
                        )
                    })
            }
        }
    };
}

resolve_float!(f32);
resolve_float!(f64);

resolve_unsigned!(u8);
resolve_unsigned!(u16);
resolve_unsigned!(u32);
resolve_unsigned!(u64);
resolve_unsigned!(u128);
resolve_unsigned!(usize);

resolve_signed!(i8);
resolve_signed!(i16);
resolve_signed!(i32);
resolve_signed!(i64);
resolve_signed!(i128);
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

// TODO: This could be solved with NoConfig instead.
// Implement Resolvable for all functions taking 4 or less arguments.
// (They will all fail to deserialize, but at least we can call resolve() on them)
// We could consider increasing that? It would probably increase compilation time, and clutter the
// Resolvable doc page. Maybe behind a feature if people really need it?
// (Ideally we wouldn't need it and we'd have a blanket implementation instead, but that may
// require specialization.)
impl_fn_from_config!(Resolvable (D C B A));

#[cfg(test)]
mod tests {
    use crate::{
        builder::{Config, Context},
        utils::markup::StyledString,
    };

    use super::Resolvable;

    fn check_resolves_from_conf<R>(config: Config, result: R)
    where
        R: Resolvable + PartialEq + std::fmt::Debug + 'static,
    {
        let context = Context::new();
        assert_eq!(result, context.resolve::<R>(&config).unwrap());
    }

    fn check_resolves_from_any<T, R>(value: T, result: R)
    where
        T: Clone + Send + Sync + 'static,
        R: Resolvable + PartialEq + std::fmt::Debug + 'static,
    {
        let mut context = Context::new();
        context.store("foo", value);
        let config = Config::String("$foo".into());
        assert_eq!(result, context.resolve::<R>(&config).unwrap());
    }

    #[test]
    fn test_integers() {
        fn check_integer_types<T>(result: T)
        where
            T: Clone + Send + Sync + 'static + std::fmt::Debug + PartialEq + Resolvable,
        {
            check_resolves_from_any(1usize, result.clone());
            check_resolves_from_any(1u8, result.clone());
            check_resolves_from_any(1u16, result.clone());
            check_resolves_from_any(1u32, result.clone());
            check_resolves_from_any(1u64, result.clone());
            check_resolves_from_any(1u128, result.clone());
            check_resolves_from_any(1isize, result.clone());
            check_resolves_from_any(1i8, result.clone());
            check_resolves_from_any(1i16, result.clone());
            check_resolves_from_any(1i32, result.clone());
            check_resolves_from_any(1i64, result.clone());
            check_resolves_from_any(1i128, result.clone());

            check_resolves_from_conf(serde_json::json!(1), result.clone());
        }

        check_integer_types(1usize);
        check_integer_types(1u8);
        check_integer_types(1u16);
        check_integer_types(1u32);
        check_integer_types(1u64);
        check_integer_types(1u128);
        check_integer_types(1isize);
        check_integer_types(1i8);
        check_integer_types(1i16);
        check_integer_types(1i32);
        check_integer_types(1i64);
        check_integer_types(1i128);
    }

    #[test]
    fn test_floats() {
        check_resolves_from_any(1.0f32, 1.0f32);
        check_resolves_from_any(1.0f32, 1.0f64);
        check_resolves_from_any(1.0f64, 1.0f32);
        check_resolves_from_any(1.0f64, 1.0f64);
        check_resolves_from_conf(serde_json::json!(1), 1.0f32);
        check_resolves_from_conf(serde_json::json!(1), 1.0f64);
        check_resolves_from_conf(serde_json::json!(1.0), 1.0f32);
        check_resolves_from_conf(serde_json::json!(1.0), 1.0f64);
    }

    #[test]
    fn test_vec() {
        // Vec to Vec
        check_resolves_from_any(vec![1u32, 2, 3], vec![1u32, 2, 3]);
        // Vec to Array
        check_resolves_from_any(vec![1u32, 2, 3], [1u32, 2, 3]);
        // Array to Array
        check_resolves_from_any([1u32, 2, 3], [1u32, 2, 3]);

        // Both array and Vec can resolve from a config array.
        check_resolves_from_conf(serde_json::json!([1, 2, 3]), [1u32, 2, 3]);
        check_resolves_from_conf(serde_json::json!([1, 2, 3]), vec![1u32, 2, 3]);
    }

    #[test]
    fn test_option() {
        check_resolves_from_any(Some(42u32), Some(42u32));
        check_resolves_from_any(42u32, Some(42u32));
        check_resolves_from_conf(serde_json::json!(42), Some(42u32));
        check_resolves_from_conf(serde_json::json!(null), None::<u32>);
    }

    #[test]
    fn test_box() {
        check_resolves_from_any(Box::new(42u32), Box::new(42u32));
        check_resolves_from_any(42u32, Box::new(42u32));
        check_resolves_from_conf(serde_json::json!(42), Box::new(42u32));
    }

    #[test]
    fn test_arc() {
        use std::sync::Arc;
        check_resolves_from_any(Arc::new(42u32), Arc::new(42u32));
        check_resolves_from_any(42u32, Arc::new(42u32));
        check_resolves_from_conf(serde_json::json!(42), Arc::new(42u32));
    }

    #[test]
    fn test_rgb() {
        use crate::style::Rgb;
        // We can resolve both u8 and f32 from either u8 or f32 stored.
        check_resolves_from_any(Rgb::new(0u8, 0u8, 255u8), Rgb::new(0u8, 0u8, 255u8));
        check_resolves_from_any(Rgb::new(0f32, 0f32, 1f32), Rgb::new(0u8, 0u8, 255u8));
        check_resolves_from_any(Rgb::new(0u8, 0u8, 255u8), Rgb::new(0f32, 0f32, 1f32));
        check_resolves_from_any(Rgb::new(0f32, 0f32, 1f32), Rgb::new(0f32, 0f32, 1f32));

        // We can resolve both u8 and f32 from either integers or floats in json.
        check_resolves_from_conf(serde_json::json!([0, 0, 255]), Rgb::new(0u8, 0u8, 255u8));
        check_resolves_from_conf(serde_json::json!([0, 0, 255]), Rgb::new(0f32, 0f32, 1f32));
        check_resolves_from_conf(serde_json::json!([0, 0, 1.0]), Rgb::new(0f32, 0f32, 1f32));
        check_resolves_from_conf(serde_json::json!([0, 0, 1.0]), Rgb::new(0u8, 0u8, 255u8));

        check_resolves_from_conf(serde_json::json!("blue"), Rgb::blue());
        check_resolves_from_conf(serde_json::json!("#0000FF"), Rgb::blue());
        check_resolves_from_conf(serde_json::json!("0x0000FF"), Rgb::blue());
        check_resolves_from_conf(serde_json::json!({"r": 0, "g": 0, "b": 255}), Rgb::blue());
    }

    #[test]
    fn test_styled_string() {
        check_resolves_from_any(String::from("foo"), StyledString::plain("foo"));
        check_resolves_from_any(StyledString::plain("foo"), StyledString::plain("foo"));
        check_resolves_from_conf(serde_json::json!("foo"), StyledString::plain("foo"));
    }

    #[test]
    fn test_no_config() {
        use super::NoConfig;

        // Test how NoConfig lets you store and resolve types that are not `Resolvable`.

        #[derive(Clone, PartialEq, Eq, Debug)]
        struct Foo(i32);

        check_resolves_from_any(Foo(42), NoConfig(Foo(42)));
    }
}
