//! Keys for custom palette entries.
//!
//! The palette defines a fixed set of colors ([`PaletteColor`]) and styles ([`PaletteStyle`])
//! used by the standard views. Views with more specific needs can define their own entries:
//!
//! * A [`ColorKey`] names a custom color.
//! * A [`StyleKey`] names a custom style.
//!
//! Each key includes a fallback, used when the current palette has no value for it. This means
//! views using custom keys work with any theme, and applications only need to define the entries
//! they want to change.
//!
//! Keys are usually defined as `static` items:
//!
//! ```rust
//! use cursive_core::style::{BaseColor, ColorKey, ColorStyle, Palette, PaletteColor};
//!
//! /// Color used for warnings.
//! pub static WARNING: ColorKey = ColorKey::new("my_crate.warning", BaseColor::Yellow.dark());
//! /// Color used for the axis of a chart: defaults to the tertiary color.
//! pub static AXIS: ColorKey = ColorKey::from_palette("my_crate.axis", PaletteColor::Tertiary);
//!
//! // Keys can be used anywhere a color is expected.
//! let style = ColorStyle::front(&WARNING);
//!
//! // Applications (or parent views) can give them a value in the palette.
//! let mut palette = Palette::default();
//! palette.set_color("my_crate.warning", BaseColor::Red.light());
//! ```
//!
//! Names can use dots to group entries in namespaces. Prefixing the name with the name of the
//! crate defining it is a good way to avoid collisions.

use super::{Color, ColorType, PaletteColor, PaletteStyle, Style, StyleType};

use std::sync::{LazyLock, Mutex};

// Use AHash instead of the slower SipHash
type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

/// Names a custom color in the palette.
///
/// See the [module documentation](self) for more information.
///
/// A key whose name is that of a standard color (`"view"`, `"highlight"`...) does not look in the
/// custom colors: it reads that standard color, like [`Palette::set_color`](crate::style::Palette::set_color) would set it, and
/// its fallback is never used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ColorKey {
    name: &'static str,
    // Invariant: this is never `ColorType::Custom`.
    // This guarantees resolving a key always terminates.
    fallback: ColorType,
}

impl ColorKey {
    /// Creates a new key, using the given color if the palette does not define this key.
    pub const fn new(name: &'static str, fallback: Color) -> Self {
        ColorKey {
            name,
            fallback: ColorType::Color(fallback),
        }
    }

    /// Creates a new key, using the given palette color if the palette does not define this key.
    pub const fn from_palette(name: &'static str, fallback: PaletteColor) -> Self {
        ColorKey {
            name,
            fallback: ColorType::Palette(fallback),
        }
    }

    /// Creates a new key, re-using the parent color if the palette does not define this key.
    pub const fn inherit_parent(name: &'static str) -> Self {
        ColorKey {
            name,
            fallback: ColorType::InheritParent,
        }
    }

    /// Returns a key for a name only known at runtime.
    ///
    /// Prefer defining keys as `static` items when possible. This is meant for names coming from
    /// configuration files.
    ///
    /// If `fallback` is itself a custom key, the fallback from that key is used instead.
    ///
    /// Note: each new combination of name and fallback is kept in memory until the end of the
    /// program. Calling this again with the same arguments returns the same key.
    pub fn intern(name: &str, fallback: ColorType) -> &'static ColorKey {
        static KEYS: Interner<ColorKey> = Interner::new();

        let fallback = match fallback {
            ColorType::Custom(key) => key.fallback,
            other => other,
        };

        KEYS.intern(
            name,
            |key| key.fallback == fallback,
            |name| ColorKey { name, fallback },
        )
    }

    /// Returns the name of this key.
    ///
    /// This is the name to use when defining the value in the palette.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the color to use when the palette does not define this key.
    ///
    /// This is never a `ColorType::Custom`.
    pub const fn fallback(&self) -> ColorType {
        self.fallback
    }
}

/// Names a custom style in the palette.
///
/// See the [module documentation](self) for more information.
///
/// A key whose name is that of a standard style (`"highlight"`, `"title_primary"`...) does not
/// look in the custom styles: it reads that standard style, like [`Palette::set_style`](crate::style::Palette::set_style) would set
/// it, and its fallback is never used.
///
/// # Examples
///
/// ```rust
/// use cursive_core::style::{Effect, PaletteStyle, Style, StyleKey, StyleType};
///
/// pub static HEADER: StyleKey =
///     StyleKey::from_palette("my_crate.header", PaletteStyle::TitlePrimary);
///
/// let style = StyleType::from(&HEADER);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StyleKey {
    name: &'static str,
    // Invariant: this is never `StyleType::Custom`.
    fallback: StyleType,
}

impl StyleKey {
    /// Creates a new key, using the given style if the palette does not define this key.
    pub const fn new(name: &'static str, fallback: Style) -> Self {
        StyleKey {
            name,
            fallback: StyleType::Style(fallback),
        }
    }

    /// Creates a new key, using the given palette style if the palette does not define this key.
    pub const fn from_palette(name: &'static str, fallback: PaletteStyle) -> Self {
        StyleKey {
            name,
            fallback: StyleType::Palette(fallback),
        }
    }

    /// Returns a key for a name only known at runtime.
    ///
    /// Prefer defining keys as `static` items when possible. This is meant for names coming from
    /// configuration files.
    ///
    /// If `fallback` is itself a custom key, the fallback from that key is used instead.
    ///
    /// Note: each new combination of name and fallback is kept in memory until the end of the
    /// program. Calling this again with the same arguments returns the same key.
    pub fn intern(name: &str, fallback: StyleType) -> &'static StyleKey {
        static KEYS: Interner<StyleKey> = Interner::new();

        let fallback = match fallback {
            StyleType::Custom(key) => key.fallback,
            other => other,
        };

        KEYS.intern(
            name,
            |key| key.fallback == fallback,
            |name| StyleKey { name, fallback },
        )
    }

    /// Returns the name of this key.
    ///
    /// This is the name to use when defining the value in the palette.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the style to use when the palette does not define this key.
    ///
    /// This is never a `StyleType::Custom`.
    pub const fn fallback(&self) -> StyleType {
        self.fallback
    }
}

/// Keeps keys created at runtime alive for the rest of the program.
struct Interner<T: 'static> {
    keys: LazyLock<Mutex<HashMap<&'static str, Vec<&'static T>>>>,
}

impl<T> Interner<T> {
    const fn new() -> Self {
        Interner {
            keys: LazyLock::new(|| Mutex::new(HashMap::default())),
        }
    }

    fn intern(
        &self,
        name: &str,
        matches: impl Fn(&T) -> bool,
        make: impl FnOnce(&'static str) -> T,
    ) -> &'static T {
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());

        if let Some((&name, candidates)) = keys.get_key_value(name) {
            if let Some(key) = candidates.iter().find(|key| matches(key)) {
                return key;
            }
            // Same name, new fallback: re-use the name we already leaked.
            let key: &'static T = Box::leak(Box::new(make(name)));
            keys.get_mut(name).unwrap().push(key);
            return key;
        }

        let name: &'static str = Box::leak(name.to_string().into_boxed_str());
        let key: &'static T = Box::leak(Box::new(make(name)));
        keys.insert(name, vec![key]);
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{BaseColor, ColorPair, ConcreteStyle, Effect, Palette};

    static WARNING: ColorKey = ColorKey::new("test.warning", Color::Dark(BaseColor::Yellow));
    static AXIS: ColorKey = ColorKey::from_palette("test.axis", PaletteColor::Tertiary);
    static NOTHING: ColorKey = ColorKey::inherit_parent("test.nothing");
    static HEADER: StyleKey = StyleKey::from_palette("test.header", PaletteStyle::TitlePrimary);

    #[test]
    fn color_key_falls_back() {
        let palette = Palette::retro();
        let previous = Color::TerminalDefault;

        assert_eq!(
            ColorType::from(&WARNING).resolve(&palette, previous),
            Color::Dark(BaseColor::Yellow)
        );
        assert_eq!(
            ColorType::from(&AXIS).resolve(&palette, previous),
            palette[PaletteColor::Tertiary]
        );
        assert_eq!(
            ColorType::from(&NOTHING).resolve(&palette, Color::Light(BaseColor::Green)),
            Color::Light(BaseColor::Green)
        );
    }

    #[test]
    fn color_key_uses_palette() {
        let mut palette = Palette::retro();
        palette.set_color("test.warning", Color::Rgb(1, 2, 3));
        palette.set_custom_color(&AXIS, Color::Rgb(4, 5, 6));

        let previous = Color::TerminalDefault;
        assert_eq!(
            ColorType::from(&WARNING).resolve(&palette, previous),
            Color::Rgb(1, 2, 3)
        );
        assert_eq!(
            ColorType::from(&AXIS).resolve(&palette, previous),
            Color::Rgb(4, 5, 6)
        );

        // Dotted names are stored as namespaces.
        assert_eq!(palette.custom("test.warning"), Some(&Color::Rgb(1, 2, 3)));
        assert_eq!(palette.custom("test"), None);
        assert_eq!(palette.custom("test.warning.more"), None);
        assert_eq!(palette.custom("other.warning"), None);
    }

    #[test]
    fn namespace_can_still_be_merged() {
        let mut palette = Palette::retro();
        palette.set_color("night.view", Color::Rgb(0, 0, 0));
        palette.set_color("night.test.warning", Color::Rgb(9, 9, 9));

        let night = palette.merge("night");
        assert_eq!(night[PaletteColor::View], Color::Rgb(0, 0, 0));
        assert_eq!(night.custom("test.warning"), Some(&Color::Rgb(9, 9, 9)));
        assert_eq!(palette.custom("test.warning"), None);
    }

    #[test]
    fn style_key() {
        let mut palette = Palette::retro();
        let previous = ConcreteStyle::terminal_default();

        // Without an entry in the palette, we use the fallback.
        let fallback = StyleType::from(PaletteStyle::TitlePrimary).resolve(&palette);
        assert_eq!(StyleType::from(&HEADER).resolve(&palette), fallback);

        // Custom styles can use custom colors.
        palette.set_color("test.warning", Color::Rgb(1, 2, 3));
        palette.set_style(
            "test.header",
            Style::from(ColorType::from(&WARNING)).combine(Effect::Bold),
        );
        let style = StyleType::from(&HEADER)
            .resolve(&palette)
            .resolve(&palette, previous);
        assert_eq!(
            style.color,
            ColorPair {
                front: Color::Rgb(1, 2, 3),
                back: Color::TerminalDefault,
            }
        );
        assert!(style.effects.contains(Effect::Bold));

        // Standard styles can be set by name too.
        palette.set_style("highlight", Style::from(Effect::Italic));
        assert_eq!(
            palette[PaletteStyle::Highlight],
            Style::from(Effect::Italic)
        );
        assert_eq!(palette.custom_style("highlight"), None);
    }

    #[cfg(feature = "toml")]
    #[test]
    fn toml_custom_entries() {
        let theme = crate::theme::load_toml(
            r##"
            [colors]
            view = "black"

            [colors.test]
            warning = "#010203"

            [styles.highlight]
            effects = ["underline"]
            front = "inherit_parent"
            back = "inherit_parent"

            [styles.test.header]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"
            "##,
        )
        .unwrap();
        let palette = theme.palette;

        assert_eq!(palette[PaletteColor::View], Color::Dark(BaseColor::Black));
        assert_eq!(palette.custom("test.warning"), Some(&Color::Rgb(1, 2, 3)));
        assert_eq!(
            palette[PaletteStyle::Highlight],
            Style::from(Effect::Underline)
        );
        assert_eq!(
            palette.custom_style("test.header"),
            Some(&Style::from(Color::Dark(BaseColor::Red)).combine(Effect::Bold))
        );
    }

    #[cfg(feature = "toml")]
    #[test]
    fn toml_style_with_custom_color() {
        let theme = crate::theme::load_toml(
            r##"
            [colors.chart]
            axis = "#010203"

            [styles.test_title]
            effects = ["bold"]
            front = { custom = "chart.axis" }
            back = "inherit_parent"
            "##,
        )
        .unwrap();
        let mut palette = theme.palette;
        let previous = ConcreteStyle::terminal_default();

        let style = *palette.custom_style("test_title").unwrap();
        assert_eq!(
            style.color.front,
            ColorType::Custom(ColorKey::intern("chart.axis", ColorType::InheritParent))
        );
        let resolved = StyleType::Style(style)
            .resolve(&palette)
            .resolve(&palette, previous);
        assert_eq!(resolved.color.front, Color::Rgb(1, 2, 3));
        assert!(resolved.effects.contains(Effect::Bold));

        // The style follows later changes of the palette.
        palette.set_color("chart.axis", Color::Rgb(7, 8, 9));
        let resolved = StyleType::Style(style)
            .resolve(&palette)
            .resolve(&palette, previous);
        assert_eq!(resolved.color.front, Color::Rgb(7, 8, 9));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn toml_style_custom_color_fallback() {
        let theme = crate::theme::load_toml(
            r##"
            [styles.test_fallback]
            effects = []
            front = { custom = { name = "test.undefined_in_toml", fallback = "#0a0b0c" } }
            back = { custom = { name = "test.undefined_palette", fallback = "view" } }

            [styles.test_default]
            effects = []
            front = { custom = { name = "test.undefined_in_toml" } }
            back = "inherit_parent"
            "##,
        )
        .unwrap();
        let palette = theme.palette;
        let previous = ConcreteStyle {
            color: ColorPair {
                front: Color::Rgb(1, 1, 1),
                back: Color::Rgb(2, 2, 2),
            },
            ..ConcreteStyle::terminal_default()
        };

        // The key is not defined: use the fallback.
        let style = *palette.custom_style("test_fallback").unwrap();
        let resolved = style.resolve(&palette, previous);
        assert_eq!(resolved.color.front, Color::Rgb(10, 11, 12));
        assert_eq!(resolved.color.back, palette[PaletteColor::View]);

        // Without a fallback, the parent color is inherited.
        let style = *palette.custom_style("test_default").unwrap();
        let resolved = style.resolve(&palette, previous);
        assert_eq!(resolved.color.front, Color::Rgb(1, 1, 1));

        // The same key with a different fallback is still the same palette entry.
        let mut palette = palette;
        palette.set_color("test.undefined_in_toml", Color::Rgb(5, 5, 5));
        for name in ["test_fallback", "test_default"] {
            let style = *palette.custom_style(name).unwrap();
            let resolved = style.resolve(&palette, previous);
            assert_eq!(resolved.color.front, Color::Rgb(5, 5, 5));
        }
    }

    #[cfg(feature = "toml")]
    #[test]
    fn toml_style_malformed_custom_color() {
        let theme = crate::theme::load_toml(
            r##"
            [styles.test_bad_type]
            effects = []
            front = { custom = 3 }
            back = "inherit_parent"

            [styles.test_bad_key]
            effects = []
            front = { custm = "chart.axis" }
            back = "inherit_parent"

            [styles.test_bad_fallback]
            effects = []
            front = { custom = { name = "chart.axis", fallback = "no such color" } }
            back = "inherit_parent"

            [styles.test_missing_name]
            effects = []
            front = { custom = { fallback = "red" } }
            back = "inherit_parent"

            [styles.test_extra_key]
            effects = []
            front = { custom = "chart.axis", fallback = "red" }
            back = "inherit_parent"

            # Unknown plain strings are not custom keys.
            [styles.test_unknown_string]
            effects = []
            front = "chart.axis"
            back = "inherit_parent"
            "##,
        )
        .unwrap();
        let palette = theme.palette;

        for name in [
            "test_bad_type",
            "test_bad_key",
            "test_bad_fallback",
            "test_missing_name",
            "test_extra_key",
            "test_unknown_string",
        ] {
            assert_eq!(palette.custom_style(name), None, "{name}");
        }
    }

    #[test]
    fn a_key_named_like_a_standard_color_reads_the_standard_color() {
        static VIEW: ColorKey = ColorKey::new("view", Color::Rgb(9, 9, 9));
        let mut palette = Palette::retro();
        palette.set_color("view", Color::Rgb(1, 2, 3));
        let previous = Color::TerminalDefault;
        assert_eq!(
            ColorType::from(&VIEW).resolve(&palette, previous),
            Color::Rgb(1, 2, 3)
        );
        // Also with the other spelling of the name, as `set_color` accepts it.
        let key = ColorKey::intern("Background", ColorType::InheritParent);
        palette.set_color("Background", Color::Rgb(4, 5, 6));
        assert_eq!(
            ColorType::from(key).resolve(&palette, previous),
            Color::Rgb(4, 5, 6)
        );
        // `custom` only looks at the custom colors.
        assert_eq!(palette.custom("view"), None);
        // Only a whole key is a standard name.
        palette.set_color("test.view", Color::Rgb(7, 7, 7));
        let key = ColorKey::intern("test.view", ColorType::InheritParent);
        assert_eq!(
            ColorType::from(key).resolve(&palette, previous),
            Color::Rgb(7, 7, 7)
        );
    }

    #[test]
    fn a_key_named_like_a_standard_style_reads_the_standard_style() {
        static HIGHLIGHT: StyleKey = StyleKey::new("highlight", Style::none());
        let mut palette = Palette::retro();
        let bold = Style::from(Effect::Bold);
        palette.set_style("highlight", bold);
        assert_eq!(StyleType::from(&HIGHLIGHT).resolve(&palette), bold);
        assert_eq!(palette.custom_style("highlight"), None);

        palette.set_style("test.highlight", Style::from(Effect::Italic));
        let key = StyleKey::intern("test.highlight", StyleType::inherit_parent());
        assert_eq!(
            StyleType::from(key).resolve(&palette),
            Style::from(Effect::Italic)
        );
    }

    #[cfg(feature = "toml")]
    #[test]
    fn toml_custom_color_named_like_a_standard_color() {
        let theme = crate::theme::load_toml(
            r##"
            [colors]
            view = "#010203"

            [styles.mine]
            effects = ["bold"]
            front = { custom = "view" }
            back = { custom = { name = "highlight", fallback = "#000000" } }
            "##,
        )
        .unwrap();
        let palette = &theme.palette;
        let style = palette.custom_style("mine").unwrap();
        let previous = Color::TerminalDefault;
        assert_eq!(
            style.color.front.resolve(palette, previous),
            Color::Rgb(1, 2, 3)
        );
        assert_eq!(
            style.color.back.resolve(palette, previous),
            palette[PaletteColor::Highlight]
        );
    }

    #[test]
    fn interned_keys() {
        let a = ColorKey::intern("test.interned", ColorType::InheritParent);
        let b = ColorKey::intern("test.interned", ColorType::InheritParent);
        assert!(std::ptr::eq(a, b));
        assert_eq!(a.name(), "test.interned");

        // A different fallback is a different key.
        let c = ColorKey::intern("test.interned", PaletteColor::Primary.into());
        assert_ne!(a, c);
        assert!(std::ptr::eq(a.name(), c.name()));

        // A custom key is not a valid fallback: we use its own fallback instead.
        let d = ColorKey::intern("test.chained", ColorType::from(&AXIS));
        assert_eq!(d.fallback(), ColorType::Palette(PaletteColor::Tertiary));

        let e = StyleKey::intern("test.interned", StyleType::from(&HEADER));
        assert_eq!(e.fallback(), StyleType::Palette(PaletteStyle::TitlePrimary));
    }
}
