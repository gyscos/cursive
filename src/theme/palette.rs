use super::Color;
use enum_map::{enum_map, Enum, EnumMap};
use log::warn;
use toml;

use hashbrown::HashMap;
use std::ops::{Index, IndexMut};

/// Color configuration for the application.
///
/// Assign each color role an actual color.
///
/// It implements `Index` and `IndexMut` to access and modify this mapping:
///
/// # Example
///
/// ```rust
/// # use cursive::theme::Palette;
/// use cursive::theme::PaletteColor::*;
/// use cursive::theme::Color::*;
/// use cursive::theme::BaseColor::*;
///
/// let mut palette = Palette::default();
///
/// assert_eq!(palette[Background], Dark(Blue));
/// palette[Shadow] = Light(Red);
/// ```
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Palette {
    basic: EnumMap<PaletteColor, Color>,
    custom: HashMap<String, PaletteNode>,
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

// We can alter existing color if needed (but why?...)
impl IndexMut<PaletteColor> for Palette {
    fn index_mut(&mut self, palette_color: PaletteColor) -> &mut Color {
        &mut self.basic[palette_color]
    }
}

impl Palette {
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
    pub fn merge(&self, namespace: &str) -> Palette {
        let mut result = self.clone();

        if let Some(&PaletteNode::Namespace(ref palette)) =
            self.custom.get(namespace)
        {
            // Merge `result` and `palette`
            for (key, value) in palette.iter() {
                match *value {
                    PaletteNode::Color(color) => result.set_color(key, color),
                    PaletteNode::Namespace(ref map) => {
                        result.add_namespace(key, map.clone())
                    }
                }
            }
        }

        result
    }

    /// Sets the color for the given key.
    ///
    /// This will update either the basic palette or the custom values.
    pub fn set_color(&mut self, key: &str, color: Color) {
        use crate::theme::PaletteColor::*;

        match key {
            "background" => self.basic[Background] = color,
            "shadow" => self.basic[Shadow] = color,
            "view" => self.basic[View] = color,
            "primary" => self.basic[Primary] = color,
            "secondary" => self.basic[Secondary] = color,
            "tertiary" => self.basic[Tertiary] = color,
            "title_primary" => self.basic[TitlePrimary] = color,
            "title_secondary" => self.basic[TitleSecondary] = color,
            "highlight" => self.basic[Highlight] = color,
            "highlight_inactive" => self.basic[HighlightInactive] = color,
            other => {
                self.custom
                    .insert(other.to_string(), PaletteNode::Color(color));
            }
        }
    }

    /// Adds a color namespace to this palette.
    pub fn add_namespace(
        &mut self,
        key: &str,
        namespace: HashMap<String, PaletteNode>,
    ) {
        self.custom
            .insert(key.to_string(), PaletteNode::Namespace(namespace));
    }
}

/// Returns the default palette for a cursive application.
///
/// * `Background` => `Dark(Blue)`
/// * `Shadow` => `Dark(Black)`
/// * `View` => `Dark(White)`
/// * `Primary` => `Dark(Black)`
/// * `Secondary` => `Dark(Blue)`
/// * `Tertiary` => `Dark(White)`
/// * `TitlePrimary` => `Dark(Red)`
/// * `TitleSecondary` => `Dark(Yellow)`
/// * `Highlight` => `Dark(Red)`
/// * `HighlightInactive` => `Dark(Blue)`
impl Default for Palette {
    fn default() -> Palette {
        use self::PaletteColor::*;
        use crate::theme::BaseColor::*;
        use crate::theme::Color::*;

        Palette {
            basic: enum_map! {
                Background => Dark(Blue),
                Shadow => Dark(Black),
                View => Dark(White),
                Primary => Dark(Black),
                Secondary => Dark(Blue),
                Tertiary => Dark(White),
                TitlePrimary => Dark(Red),
                TitleSecondary => Dark(Yellow),
                Highlight => Dark(Red),
                HighlightInactive => Dark(Blue),
            },
            custom: HashMap::new(),
        }
    }
}

// Iterate over a toml
fn iterate_toml<'a>(
    table: &'a toml::value::Table,
) -> impl Iterator<Item = (&'a str, PaletteNode)> + 'a {
    table.iter().flat_map(|(key, value)| {
        let node = match value {
            toml::Value::Table(table) => {
                // This should define a new namespace
                // Treat basic colors as simple string.
                // We'll convert them back in the merge method.
                let map = iterate_toml(table)
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
                warn!(
                    "Found unexpected value in theme: {} = {:?}",
                    key, other
                );
                None
            }
        };

        node.map(|node| (key.as_str(), node))
    })
}

/// Fills `palette` with the colors from the given `table`.
pub(crate) fn load_toml(palette: &mut Palette, table: &toml::value::Table) {
    // TODO: use serde for that?
    // Problem: toml-rs doesn't do well with Enums...

    for (key, value) in iterate_toml(table) {
        match value {
            PaletteNode::Color(color) => palette.set_color(key, color),
            PaletteNode::Namespace(map) => palette.add_namespace(key, map),
        }
    }
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
}

impl PaletteColor {
    /// Given a palette, resolve `self` to a concrete color.
    pub fn resolve(self, palette: &Palette) -> Color {
        palette[self]
    }
}
