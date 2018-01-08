use super::Color;
use toml;

/// Color configuration for the application.
///
/// Assign each color role an actual color.
#[derive(Copy, Clone, Debug)]
pub struct Palette {
    /// Color used for the application background.
    pub background: Color,
    /// Color used for View shadows.
    pub shadow: Color,
    /// Color used for View backgrounds.
    pub view: Color,
    /// Primary color used for the text.
    pub primary: Color,
    /// Secondary color used for the text.
    pub secondary: Color,
    /// Tertiary color used for the text.
    pub tertiary: Color,
    /// Primary color used for title text.
    pub title_primary: Color,
    /// Secondary color used for title text.
    pub title_secondary: Color,
    /// Color used for highlighting text.
    pub highlight: Color,
    /// Color used for highlighting inactive text.
    pub highlight_inactive: Color,
}

impl Palette {
    /// Fills `self` with the colors from the given `table`.
    pub(crate) fn load(&mut self, table: &toml::value::Table) {
        load_color(&mut self.background, table.get("background"));
        load_color(&mut self.shadow, table.get("shadow"));
        load_color(&mut self.view, table.get("view"));
        load_color(&mut self.primary, table.get("primary"));
        load_color(&mut self.secondary, table.get("secondary"));
        load_color(&mut self.tertiary, table.get("tertiary"));
        load_color(&mut self.title_primary, table.get("title_primary"));
        load_color(&mut self.title_secondary, table.get("title_secondary"));
        load_color(&mut self.highlight, table.get("highlight"));
        load_color(
            &mut self.highlight_inactive,
            table.get("highlight_inactive"),
        );
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
