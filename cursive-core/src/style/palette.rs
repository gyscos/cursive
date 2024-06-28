use super::{Color, Effects, NoSuchColor, Style};
use enum_map::{enum_map, Enum, EnumMap};
#[cfg(feature = "toml")]
use log::warn;

use std::ops::{Index, IndexMut};
use std::str::FromStr;

// Use AHash instead of the slower SipHash
type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

/// Color configuration for the application.
///
/// Assign each color role an actual color.
///
/// It implements `Index` and `IndexMut` to access and modify this mapping:
///
/// It also implements [`Extend`] to update a batch of colors at
/// once.
///
/// # Example
///
/// ```rust
/// # use cursive_core::style::Palette;
/// use cursive_core::style::{BaseColor::*, Color::*, PaletteColor::*};
///
/// let mut palette = Palette::default();
///
/// assert_eq!(palette[Background], Dark(Blue));
/// palette[Shadow] = Light(Red);
/// assert_eq!(palette[Shadow], Light(Red));
///
/// let colors = vec![(Shadow, Dark(Green)), (Primary, Light(Blue))];
/// palette.extend(colors);
/// assert_eq!(palette[Shadow], Dark(Green));
/// assert_eq!(palette[Primary], Light(Blue));
/// ```
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Palette {
    basic: EnumMap<PaletteColor, Color>,
    custom: HashMap<String, PaletteNode>,
    styles: EnumMap<PaletteStyle, Style>,
}

/// A node in the palette tree.
///
/// This describes a value attached to a custom keyword in the palette.
///
/// This can either be a color, or a nested namespace with its own mapping.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum PaletteNode {
    /// A single color.
    Color(Color),
    /// A group of values bundled in the same namespace.
    ///
    /// Namespaces can be merged in the palette with `Palette::merge`.
    Namespace(HashMap<String, PaletteNode>),
}

// Basic usage: only use basic colors
impl Index<PaletteColor> for Palette {
    type Output = Color;

    fn index(&self, palette_color: PaletteColor) -> &Color {
        &self.basic[palette_color]
    }
}

impl Index<PaletteStyle> for Palette {
    type Output = Style;

    fn index(&self, palette_style: PaletteStyle) -> &Style {
        &self.styles[palette_style]
    }
}

impl IndexMut<PaletteColor> for Palette {
    fn index_mut(&mut self, palette_color: PaletteColor) -> &mut Color {
        &mut self.basic[palette_color]
    }
}

impl IndexMut<PaletteStyle> for Palette {
    fn index_mut(&mut self, palette_style: PaletteStyle) -> &mut Style {
        &mut self.styles[palette_style]
    }
}

fn default_styles() -> EnumMap<PaletteStyle, Style> {
    use self::PaletteStyle::*;
    use crate::style::{ColorStyle, Effect};

    enum_map! {
        Shadow => ColorStyle::shadow().into(),
        Primary => ColorStyle::primary().into(),
        Secondary => ColorStyle::secondary().into(),
        Tertiary => ColorStyle::tertiary().into(),
        View => ColorStyle::view().into(),
        Background => ColorStyle::background().into(),
        TitlePrimary => ColorStyle::title_primary().into(),
        TitleSecondary => ColorStyle::title_secondary().into(),
        Highlight => Style {
            color: ColorStyle::highlight().invert(),
            effects: Effects::only(Effect::Reverse),
        },
        HighlightInactive => Style {
            color: ColorStyle::highlight_inactive().invert(),
            effects: Effects::only(Effect::Reverse),
        },
        EditableText => Style {
            color: ColorStyle::secondary(),
            effects: Effects::only(Effect::Reverse),
        },
        EditableTextCursor => ColorStyle::secondary().into(),
        EditableTextInactive => ColorStyle::secondary().into(),
    }
}

impl Palette {
    /// Returns a bi-color palette using the terminal's default background and
    /// text color for everything.
    pub fn terminal_default() -> Self {
        use self::PaletteColor::*;
        use crate::style::Color::TerminalDefault;

        Palette {
            basic: enum_map! {
                Background => TerminalDefault,
                Shadow => TerminalDefault,
                View => TerminalDefault,
                Primary => TerminalDefault,
                Secondary => TerminalDefault,
                Tertiary => TerminalDefault,
                TitlePrimary => TerminalDefault,
                TitleSecondary => TerminalDefault,
                Highlight => TerminalDefault,
                HighlightInactive => TerminalDefault,
                HighlightText => TerminalDefault,
            },
            custom: HashMap::default(),
            styles: default_styles(),
        }
    }

    /// Returns the palette for a retro look, similar to dialog.
    ///
    /// * `Background` => `Dark(Blue)`
    /// * `Shadow` => `Dark(Black)`
    /// * `View` => `Dark(White)`
    /// * `Primary` => `Dark(Black)`
    /// * `Secondary` => `Dark(Blue)`
    /// * `Tertiary` => `Light(White)`
    /// * `TitlePrimary` => `Dark(Red)`
    /// * `TitleSecondary` => `Dark(Yellow)`
    /// * `Highlight` => `Dark(Red)`
    /// * `HighlightInactive` => `Dark(Blue)`
    /// * `HighlightText` => `Dark(White)`
    pub fn retro() -> Self {
        use self::PaletteColor::*;
        use crate::style::BaseColor::*;
        use crate::style::Color::*;

        Palette {
            basic: enum_map! {
                Background => Dark(Blue),
                Shadow => Dark(Black),
                View => Dark(White),
                Primary => Dark(Black),
                Secondary => Dark(Blue),
                Tertiary => Light(White),
                TitlePrimary => Dark(Red),
                TitleSecondary => Light(Blue),
                Highlight => Dark(Red),
                HighlightInactive => Dark(Blue),
                HighlightText => Dark(White),
            },
            custom: HashMap::default(),
            styles: default_styles(),
        }
    }

    /// Returns a custom color from this palette.
    ///
    /// Returns `None` if the given key was not found.
    pub fn custom<'a>(&'a self, key: &str) -> Option<&'a Color> {
        self.custom.get(key).and_then(|node| {
            if let PaletteNode::Color(ref color) = *node {
                Some(color)
            } else {
                None
            }
        })
    }

    /// Returns a new palette where the given namespace has been merged.
    ///
    /// All values in the namespace will override previous values.
    #[must_use]
    pub fn merge(&self, namespace: &str) -> Self {
        let mut result = self.clone();

        if let Some(PaletteNode::Namespace(palette)) = self.custom.get(namespace) {
            // Merge `result` and `palette`
            for (key, value) in palette.iter() {
                match *value {
                    PaletteNode::Color(color) => result.set_color(key, color),
                    PaletteNode::Namespace(ref map) => result.add_namespace(key, map.clone()),
                }
            }
        }

        result
    }

    /// Sets the color for the given key.
    ///
    /// This will update either the basic palette or the custom values.
    pub fn set_color(&mut self, key: &str, color: Color) {
        if self.set_basic_color(key, color).is_err() {
            self.custom
                .insert(key.to_string(), PaletteNode::Color(color));
        }
    }

    /// Sets a basic color from its name.
    ///
    /// Returns `Err(())` if `key` is not a known `PaletteColor`.
    pub fn set_basic_color(
        &mut self,
        key: &str,
        color: Color,
    ) -> Result<(), crate::style::NoSuchColor> {
        PaletteColor::from_str(key).map(|c| self.basic[c] = color)
    }

    /// Adds a color namespace to this palette.
    pub fn add_namespace(&mut self, key: &str, namespace: HashMap<String, PaletteNode>) {
        self.custom
            .insert(key.to_string(), PaletteNode::Namespace(namespace));
    }

    /// Fills `palette` with the colors from the given `table`.
    #[cfg(feature = "toml")]
    pub(crate) fn load_toml(&mut self, table: &toml::value::Table) {
        // TODO: use serde for that?
        // Problem: toml-rs doesn't do well with Enums...

        for (key, value) in iterate_toml_colors(table) {
            match value {
                PaletteNode::Color(color) => self.set_color(key, color),
                PaletteNode::Namespace(map) => self.add_namespace(key, map),
            }
        }
    }

    /// Fills `palette` with the colors from the given `table`.
    #[cfg(feature = "toml")]
    pub(crate) fn load_toml_styles(&mut self, table: &toml::value::Table) {
        // TODO: use serde for that?
        for (key, value) in table {
            // Find out which palette style this defines.
            let key = match key.parse() {
                Ok(key) => key,
                _ => {
                    log::warn!("Found unknown palette style: `{key}`.");
                    continue;
                }
            };

            let value = match Style::parse(value) {
                Some(value) => value,
                _ => {
                    log::warn!("Could not parse style: `{value}`.");
                    continue;
                }
            };

            self.styles[key] = value;
        }
    }
}

impl Extend<(PaletteColor, Color)> for Palette {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (PaletteColor, Color)>,
    {
        for (k, v) in iter {
            (*self)[k] = v;
        }
    }
}

/// Currently returns the retro palette.
impl Default for Palette {
    fn default() -> Palette {
        Palette::retro()
    }
}

// Iterate over a toml
#[cfg(feature = "toml")]
fn iterate_toml_colors(table: &toml::value::Table) -> impl Iterator<Item = (&str, PaletteNode)> {
    table.iter().flat_map(|(key, value)| {
        let node = match value {
            toml::Value::Table(table) => {
                // This should define a new namespace
                // Treat basic colors as simple string.
                // We'll convert them back in the merge method.
                let map = iterate_toml_colors(table)
                    .map(|(key, value)| (key.to_string(), value))
                    .collect();
                // Should we only return something if it's non-empty?
                Some(PaletteNode::Namespace(map))
            }
            toml::Value::Array(colors) => {
                // This should be a list of colors - just pick the first valid one.
                colors
                    .iter()
                    .flat_map(toml::Value::as_str)
                    .flat_map(Color::parse)
                    .map(PaletteNode::Color)
                    .next()
            }
            toml::Value::String(color) => {
                // This describe a new color - easy!
                Color::parse(color).map(PaletteNode::Color)
            }
            other => {
                // Other - error?
                warn!("Found unexpected value in theme: {} = {:?}", key, other);
                None
            }
        };

        node.map(|node| (key.as_str(), node))
    })
}

/// Color entry in a palette.
///
/// Each `PaletteColor` is used for a specific role in a default application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum PaletteColor {
    /// Color used for the application background.
    Background,
    /// Color used for View shadows.
    Shadow,
    /// Color used for View backgrounds.
    View,
    /// Primary color used for the text.
    Primary,
    /// Secondary color used for the text.
    Secondary,
    /// Tertiary color used for the text.
    Tertiary,
    /// Primary color used for title text.
    TitlePrimary,
    /// Secondary color used for title text.
    TitleSecondary,
    /// Color used for highlighting text.
    Highlight,
    /// Color used for highlighting inactive text.
    HighlightInactive,
    /// Color used for highlighted text
    HighlightText,
}

/// Style entry in a palette.
///
/// This represents a color "role". The palette will resolve this to a `Style`.
///
/// For example, `PaletteStyle::Highlight` should be used when drawing highlighted text.
/// In the default palette, it will resolve to a `Style` made of:
/// * The `Reverse` effect (front and background will be swapped).
/// * A front color of `PaletteColor::Highlight` (but with the reverse effect,
///   it will become the background color).
/// * A back color of `PaletteColor::HighlightText` (will become the front color).
///
/// From there, the `PaletteColor::Highlight` and `PaletteColor::HighlightText` will be resolved to
/// concrete colors (or possibly to `InheritParent`, which will inherit the previous concrete
/// color).
///
/// To override the look of highlighted text, you can either:
/// * Change the palette entries for `PaletteColor::Highlight`/`PaletteColor::HighlightText`.
/// * Change the palette entry for `PaletteStyle::Highlight`, possibly using different palette
///   colors instead (or directly specifying a concrete color there).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum PaletteStyle {
    /// Style used for regular text.
    Primary,
    /// Style used for secondary text.
    Secondary,
    /// Style used for tertiary text.
    Tertiary,
    /// Style used for view background.
    View,
    /// Style used for application background (behind all views).
    Background,
    /// Style used for main titles.
    TitlePrimary,
    /// Style used for secondary titles.
    TitleSecondary,
    /// Style used for highlighted text.
    Highlight,
    /// Style used for inactive highlighted text.
    HighlightInactive,

    /// Style used to draw the drop shadows (1-cell border to the bottom/right
    /// of views).
    Shadow,

    /// Style used for editable text (TextArea, EditView).
    EditableText,
    /// Style used for the selected character in editable text.
    EditableTextCursor,
    /// Style used for editable text when inactive.
    EditableTextInactive,
}

impl PaletteStyle {
    /// Given a style palette, resolve `self` to a concrete style.
    pub fn resolve(self, palette: &Palette) -> Style {
        palette[self]
    }

    /// Returns an iterator on all possible palette styles.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }
}

impl PaletteColor {
    /// Given a palette, resolve `self` to a concrete color.
    pub fn resolve(self, palette: &Palette) -> Color {
        palette[self]
    }

    /// Returns an iterator on all possible palette colors.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }
}

impl FromStr for PaletteStyle {
    type Err = NoSuchColor;

    fn from_str(s: &str) -> Result<Self, NoSuchColor> {
        use PaletteStyle::*;

        Ok(match s {
            // TODO: make a macro for this?
            "Background" | "background" => Background,
            "Shadow" | "shadow" => Shadow,
            "View" | "view" => View,
            "Primary" | "primary" => Primary,
            "Secondary" | "secondary" => Secondary,
            "Tertiary" | "tertiary" => Tertiary,
            "TitlePrimary" | "title_primary" => TitlePrimary,
            "TitleSecondary" | "title_secondary" => TitleSecondary,
            "Highlight" | "highlight" => Highlight,
            "HighlightInactive" | "highlight_inactive" => HighlightInactive,
            "EditableText" | "editable_text" => EditableText,
            "EditableTextCursor" | "editable_text_cursor" => EditableTextCursor,
            "EditableTextInactive" | "editable_text_inactive" => EditableTextInactive,
            _ => return Err(NoSuchColor),
        })
    }
}

impl FromStr for PaletteColor {
    type Err = crate::style::NoSuchColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use PaletteColor::*;

        Ok(match s {
            "Background" | "background" => Background,
            "Shadow" | "shadow" => Shadow,
            "View" | "view" => View,
            "Primary" | "primary" => Primary,
            "Secondary" | "secondary" => Secondary,
            "Tertiary" | "tertiary" => Tertiary,
            "TitlePrimary" | "title_primary" => TitlePrimary,
            "TitleSecondary" | "title_secondary" => TitleSecondary,
            "Highlight" | "highlight" => Highlight,
            "HighlightInactive" | "highlight_inactive" => HighlightInactive,
            "HighlightText" | "highlight_text" => HighlightText,
            _ => return Err(crate::style::NoSuchColor),
        })
    }
}
