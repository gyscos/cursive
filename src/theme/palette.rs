use super::Color;
use toml;
use enum_map::EnumMap;

/// Color configuration for the application.
///
/// Assign each color role an actual color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Palette {
    /// Color map for this palette
    pub colors: EnumMap<PaletteColor, Color>,
}

impl Default for Palette {
    fn default() -> Self {
        use self::PaletteColor::*;
        use theme::Color::*;
        use theme::BaseColor::*;

        Palette {
            colors: enum_map!{
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
            }
        }
    }
}

impl Palette {
    /// Fills `self` with the colors from the given `table`.
    pub(crate) fn load(&mut self, table: &toml::value::Table) {
        // TODO: use serde for that?
        // Problem: toml-rs doesn't do well with Enums...
        load_color(
            &mut self.colors[PaletteColor::Background],
            table.get("background"),
        );
        load_color(
            &mut self.colors[PaletteColor::Shadow],
            table.get("shadow"),
        );
        load_color(&mut self.colors[PaletteColor::View], table.get("view"));
        load_color(
            &mut self.colors[PaletteColor::Primary],
            table.get("primary"),
        );
        load_color(
            &mut self.colors[PaletteColor::Secondary],
            table.get("secondary"),
        );
        load_color(
            &mut self.colors[PaletteColor::Tertiary],
            table.get("tertiary"),
        );
        load_color(
            &mut self.colors[PaletteColor::TitlePrimary],
            table.get("title_primary"),
        );
        load_color(
            &mut self.colors[PaletteColor::TitleSecondary],
            table.get("title_secondary"),
        );
        load_color(
            &mut self.colors[PaletteColor::Highlight],
            table.get("highlight"),
        );
        load_color(
            &mut self.colors[PaletteColor::HighlightInactive],
            table.get("highlight_inactive"),
        );
    }
}

/// Color entry in a palette.
///
/// Each ColorRole is used for a specific role in a default application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, EnumMap)]
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
        palette.colors[self]
    }
}

/// Parses `value` and fills `target` if it's a valid color.
fn load_color(target: &mut Color, value: Option<&toml::Value>) -> bool {
    if let Some(value) = value {
        match *value {
            toml::Value::String(ref value) => {
                if let Some(color) = Color::parse(value) {
                    *target = color;
                    true
                } else {
                    false
                }
            }
            toml::Value::Array(ref array) => {
                array.iter().any(|item| load_color(target, Some(item)))
            }
            _ => false,
        }
    } else {
        false
    }
}
