use super::{Color, ColorKey, Effects, NoSuchColor, Style, StyleKey};
use enum_map::{Enum, EnumMap, enum_map};
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
/// # Custom colors and styles
///
/// On top of the standard [`PaletteColor`]s and [`PaletteStyle`]s, a palette holds custom colors
/// (see [`ColorKey`]) and custom styles (see [`StyleKey`]) under arbitrary names.
///
/// Both are stored in a tree of namespaces, and are addressed the same way: dots in a name go
/// down into namespaces. `"tabs.active"` is the entry `active` in the namespace `tabs`, whether
/// it is a color ([`Palette::set_color`], [`Palette::custom`]) or a style
/// ([`Palette::set_style`], [`Palette::custom_style`]). The two trees are independent: a
/// namespace `tabs` can hold colors in one and styles in the other, and an entry `tabs.active`
/// can exist in both.
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
    custom_styles: HashMap<String, StyleNode>,
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

/// A node in the tree of custom styles of a palette.
///
/// This is to styles what [`PaletteNode`] is to colors: a style attached to a custom keyword, or
/// a nested namespace with its own mapping.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum StyleNode {
    /// A single style.
    Style(Style),
    /// A group of values bundled in the same namespace.
    ///
    /// Namespaces can be merged in the palette with `Palette::merge`.
    Namespace(HashMap<String, StyleNode>),
}

/// A tree node that is either a value or a namespace of nodes: [`PaletteNode`] or [`StyleNode`].
///
/// This lets colors and styles share the code that walks and builds the tree.
trait Node: Sized {
    fn new_namespace() -> Self;
    fn namespace(&self) -> Option<&HashMap<String, Self>>;
    fn namespace_mut(&mut self) -> Option<&mut HashMap<String, Self>>;
}

impl Node for PaletteNode {
    fn new_namespace() -> Self {
        PaletteNode::Namespace(HashMap::default())
    }

    fn namespace(&self) -> Option<&HashMap<String, Self>> {
        match self {
            PaletteNode::Namespace(map) => Some(map),
            PaletteNode::Color(_) => None,
        }
    }

    fn namespace_mut(&mut self) -> Option<&mut HashMap<String, Self>> {
        match self {
            PaletteNode::Namespace(map) => Some(map),
            PaletteNode::Color(_) => None,
        }
    }
}

impl Node for StyleNode {
    fn new_namespace() -> Self {
        StyleNode::Namespace(HashMap::default())
    }

    fn namespace(&self) -> Option<&HashMap<String, Self>> {
        match self {
            StyleNode::Namespace(map) => Some(map),
            StyleNode::Style(_) => None,
        }
    }

    fn namespace_mut(&mut self) -> Option<&mut HashMap<String, Self>> {
        match self {
            StyleNode::Namespace(map) => Some(map),
            StyleNode::Style(_) => None,
        }
    }
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
            custom_styles: HashMap::default(),
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
            custom_styles: HashMap::default(),
        }
    }

    /// Returns a custom color from this palette.
    ///
    /// Only the custom colors are looked at: a standard color (`view`, `highlight`...) is read
    /// with the `PaletteColor` index, and `custom("view")` is `None` even though
    /// [`Palette::set_color`] sets the standard color of that name. (A [`ColorKey`] with such a
    /// name does read the standard color.)
    ///
    /// The key can use dots to look into namespaces: `custom("charts.axis")` returns the color
    /// `axis` from the namespace `charts`.
    ///
    /// Returns `None` if the given key was not found, or if it is a namespace rather than a
    /// color.
    pub fn custom<'a>(&'a self, key: &str) -> Option<&'a Color> {
        match find_node(&self.custom, key)? {
            PaletteNode::Color(color) => Some(color),
            PaletteNode::Namespace(_) => None,
        }
    }

    /// Returns a custom style from this palette.
    ///
    /// The key can use dots to look into namespaces, like for [`Palette::custom`]:
    /// `custom_style("tabs.active")` returns the style `active` from the namespace `tabs`.
    ///
    /// Standard styles are not custom styles: they are read with the `PaletteStyle` index, and
    /// `custom_style("highlight")` is `None` even though [`Palette::set_style`] sets the standard
    /// style of that name. (A [`StyleKey`] with such a name does read the standard style.)
    ///
    /// Returns `None` if the given key was not found, or if it is a namespace rather than a
    /// style.
    pub fn custom_style<'a>(&'a self, key: &str) -> Option<&'a Style> {
        match find_node(&self.custom_styles, key)? {
            StyleNode::Style(style) => Some(style),
            StyleNode::Namespace(_) => None,
        }
    }

    /// Returns the value of a color key: a standard color if the whole name is one, and the
    /// custom color otherwise.
    ///
    /// This is the lookup used by [`ColorKey`]s, matching what [`Palette::set_color`] writes.
    pub(crate) fn color_for_key(&self, name: &str) -> Option<Color> {
        match PaletteColor::from_str(name) {
            Ok(color) => Some(self[color]),
            Err(_) => self.custom(name).copied(),
        }
    }

    /// Returns the value of a style key: a standard style if the whole name is one, and the
    /// custom style otherwise.
    ///
    /// This is the lookup used by [`StyleKey`]s, matching what [`Palette::set_style`] writes.
    pub(crate) fn style_for_key(&self, name: &str) -> Option<Style> {
        match PaletteStyle::from_str(name) {
            Ok(style) => Some(self[style]),
            Err(_) => self.custom_style(name).copied(),
        }
    }

    /// Returns a new palette where the given namespace has been merged.
    ///
    /// The namespace is a top-level one (`"night"`, not a dotted name). Colors and styles are
    /// both merged, each from its own tree: the namespace can exist in either or both.
    ///
    /// All values in the namespace will override previous values. For each entry in the namespace:
    ///
    /// * A color or a style is set with [`Palette::set_color`] or [`Palette::set_style`]. If its
    ///   name is a standard color or style (`view`, `highlight`...), it sets the standard one;
    ///   otherwise it becomes a top-level custom entry.
    /// * A nested namespace becomes a top-level namespace of the result, with the same name. It
    ///   replaces the whole namespace of that name if there was one (in the same tree, colors or
    ///   styles), instead of being merged into it.
    ///
    /// So with the style `night.highlight` and the nested style `night.tabs.active`, the result
    /// has a new standard `highlight` style, and the custom style `tabs.active`.
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

        if let Some(StyleNode::Namespace(styles)) = self.custom_styles.get(namespace) {
            for (key, value) in styles.iter() {
                match *value {
                    StyleNode::Style(style) => result.set_style(key, style),
                    StyleNode::Namespace(ref map) => result.add_style_namespace(key, map.clone()),
                }
            }
        }

        result
    }

    /// Sets the color for the given key.
    ///
    /// This will update either the basic palette or the custom values.
    ///
    /// For custom values, dots in the key denote namespaces: `set_color("charts.axis", color)`
    /// sets the color `axis` in the namespace `charts`, creating it if necessary.
    ///
    /// If a component of the path already holds a color, that color is replaced by the new
    /// namespace. Setting a color where there was a namespace replaces the whole namespace.
    pub fn set_color(&mut self, key: &str, color: Color) {
        if self.set_basic_color(key, color).is_err() {
            insert_node(&mut self.custom, key, PaletteNode::Color(color));
        }
    }

    /// Sets the color for the given custom key.
    ///
    /// Views using this key will now use the given color rather than the fallback from the key.
    pub fn set_custom_color(&mut self, key: &ColorKey, color: Color) {
        insert_node(&mut self.custom, key.name(), PaletteNode::Color(color));
    }

    /// Sets the style for the given key.
    ///
    /// This will update either one of the standard styles (if `key` is the name of a
    /// [`PaletteStyle`]), or the custom styles.
    ///
    /// For custom styles, dots in the key denote namespaces, like in [`Palette::set_color`]:
    /// `set_style("tabs.active", style)` sets the style `active` in the namespace `tabs`,
    /// creating it if necessary. If a component of the path already holds a style, that style is
    /// replaced by the new namespace. Setting a style where there was a namespace replaces the
    /// whole namespace. Only a whole key is checked against the standard names: `tabs.primary`
    /// is a custom style.
    pub fn set_style(&mut self, key: &str, style: Style) {
        match PaletteStyle::from_str(key) {
            Ok(key) => self.styles[key] = style,
            Err(_) => insert_node(&mut self.custom_styles, key, StyleNode::Style(style)),
        }
    }

    /// Sets the style for the given custom key.
    ///
    /// Views using this key will now use the given style rather than the fallback from the key.
    pub fn set_custom_style(&mut self, key: &StyleKey, style: Style) {
        insert_node(&mut self.custom_styles, key.name(), StyleNode::Style(style));
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

    /// Adds a style namespace to this palette.
    ///
    /// This is the equivalent of [`Palette::add_namespace`] for styles. It replaces the style or
    /// namespace that had the same name, if any.
    pub fn add_style_namespace(&mut self, key: &str, namespace: HashMap<String, StyleNode>) {
        self.custom_styles
            .insert(key.to_string(), StyleNode::Namespace(namespace));
    }

    /// Fills `palette` with the colors from the given `table`.
    ///
    /// A dot in a key is a namespace boundary at any depth (`"a.b" = ...` inside `[colors.ns]` is
    /// `ns.a.b`), like in [`Palette::set_color`].
    ///
    /// An empty table defines nothing: it creates no namespace, and replaces none.
    ///
    /// A table of colors is a namespace, and it replaces the namespace of the same name that the
    /// palette already had, rather than being merged into it. This is the same for styles (see
    /// [`Palette::load_toml_styles`]): entries of the old namespace that the table does not
    /// define are dropped. Colors given one by one (`name = "red"`, `"ns.name" = "red"`) are set
    /// without touching the rest of their namespace, and namespaces the table does not mention
    /// are left alone.
    #[cfg(feature = "toml")]
    pub(crate) fn load_toml(&mut self, table: &toml::value::Table) {
        // TODO: use serde for that?
        // Problem: toml-rs doesn't do well with Enums...

        for (key, value) in table {
            match read_toml_color(key, value) {
                TomlEntry::Node(color) => self.set_color(key, color),
                TomlEntry::Namespace(inner) => {
                    let namespace =
                        toml_namespace(key, inner, &read_toml_color, PaletteNode::Color);
                    // An empty table defines nothing: it must not create an empty namespace.
                    if !namespace.is_empty() {
                        insert_node(&mut self.custom, key, PaletteNode::Namespace(namespace));
                    }
                }
                TomlEntry::Skip => (),
            }
        }
    }

    /// Fills `palette` with the styles from the given `table`.
    ///
    /// A table is either a style or a namespace, and this is decided from its shape: it is a
    /// style if it has a `front`, `back` or `effects` key, unless one of these holds a nested
    /// style or namespace (see [`looks_like_style`]). A style that cannot be parsed is reported
    /// once, with its full key, and its content is not looked at.
    ///
    /// Other tables are namespaces: the style `[styles.charts.title]` is the style `title` in the
    /// namespace `charts`, like `[styles.charts] title = { ... }`. Like for colors
    /// (see [`Palette::load_toml`]), a namespace replaces the namespace of the same name in the
    /// palette, in the styles only: the colors of a namespace with that name are not affected.
    #[cfg(feature = "toml")]
    pub(crate) fn load_toml_styles(&mut self, table: &toml::value::Table) {
        // TODO: use serde for that?
        for (key, value) in table {
            match read_toml_style(key, value) {
                TomlEntry::Node(style) => self.set_style(key, style),
                TomlEntry::Namespace(inner) => {
                    let namespace = toml_namespace(key, inner, &read_toml_style, StyleNode::Style);
                    if !namespace.is_empty() {
                        insert_node(
                            &mut self.custom_styles,
                            key,
                            StyleNode::Namespace(namespace),
                        );
                    }
                }
                TomlEntry::Skip => (),
            }
        }
    }
}

/// What a value from a TOML theme defines.
#[cfg(feature = "toml")]
enum TomlEntry<'a, N> {
    /// A color or a style.
    Node(N),
    /// A namespace: its entries are read recursively.
    Namespace(&'a toml::value::Table),
    /// Nothing (the problem has been logged if needed).
    Skip,
}

/// Reads the entries of a TOML table as a namespace.
///
/// `path` is the full name of the table, and `read` tells what a value defines, given its full
/// name. A key with dots is a path (it goes through namespaces), at any depth, so a dot always
/// separates namespaces. The result has no empty namespace: an empty table defines nothing.
#[cfg(feature = "toml")]
fn toml_namespace<'a, V, N: Node>(
    path: &str,
    table: &'a toml::value::Table,
    read: &impl Fn(&str, &'a toml::Value) -> TomlEntry<'a, V>,
    wrap: fn(V) -> N,
) -> HashMap<String, N> {
    fn collect<'a, V>(
        path: &str,
        prefix: &str,
        table: &'a toml::value::Table,
        read: &impl Fn(&str, &'a toml::Value) -> TomlEntry<'a, V>,
        out: &mut Vec<(String, V)>,
    ) {
        for (key, value) in table {
            let relative = format!("{prefix}{key}");
            let full = format!("{path}.{relative}");
            match read(&full, value) {
                TomlEntry::Node(node) => out.push((relative, node)),
                TomlEntry::Namespace(inner) => {
                    collect(path, &format!("{relative}."), inner, read, out)
                }
                TomlEntry::Skip => (),
            }
        }
    }

    let mut nodes = Vec::new();
    collect(path, "", table, read, &mut nodes);
    let mut namespace = HashMap::default();
    for (path, node) in nodes {
        insert_node(&mut namespace, &path, wrap(node));
    }
    namespace
}

/// Reads an entry of the `[colors]` section: a color, or a namespace.
///
/// `name` is the full name of the entry.
#[cfg(feature = "toml")]
fn read_toml_color<'a>(name: &str, value: &'a toml::Value) -> TomlEntry<'a, Color> {
    let color = match value {
        toml::Value::Table(table) => return TomlEntry::Namespace(table),
        // A list of colors: just pick the first valid one.
        toml::Value::Array(colors) => colors
            .iter()
            .flat_map(toml::Value::as_str)
            .flat_map(Color::parse)
            .next(),
        toml::Value::String(color) => Color::parse(color),
        other => {
            warn!("Found unexpected value in theme: {} = {:?}", name, other);
            None
        }
    };
    color.map_or(TomlEntry::Skip, TomlEntry::Node)
}

/// Reads an entry of the `[styles]` section: a style, or a namespace.
///
/// `name` is the full name of the entry. A table is a style or a namespace depending on its
/// shape, and a standard style name is never a namespace.
#[cfg(feature = "toml")]
fn read_toml_style<'a>(name: &str, value: &'a toml::Value) -> TomlEntry<'a, Style> {
    match value {
        toml::Value::Table(inner)
            if !looks_like_style(inner) && PaletteStyle::from_str(name).is_err() =>
        {
            TomlEntry::Namespace(inner)
        }
        _ => match Style::parse(value) {
            Some(style) => TomlEntry::Node(style),
            None => {
                log::warn!("Could not parse style `{name}`: `{value}`.");
                TomlEntry::Skip
            }
        },
    }
}

/// The keys of a table describing a style.
#[cfg(feature = "toml")]
const STYLE_KEYS: [&str; 3] = ["front", "back", "effects"];

/// Tells if a table from the `styles` section describes a style, rather than a namespace.
///
/// It does if it has one of the keys of a style (`front`, `back`, `effects`), and none of their
/// values is a nested style or namespace: a namespace is allowed to hold styles with these names.
/// A `{ custom = ... }` table is a color, and a table that is neither a style nor a namespace
/// (for example a misspelled color) is a malformed color: it does not make the parent a
/// namespace.
#[cfg(feature = "toml")]
fn looks_like_style(table: &toml::value::Table) -> bool {
    table.keys().any(|key| STYLE_KEYS.contains(&key.as_str()))
        && !table
            .iter()
            .any(|(key, value)| STYLE_KEYS.contains(&key.as_str()) && is_nested(value))
}

/// Tells if a value is a style or a namespace of styles (and not a color).
#[cfg(feature = "toml")]
fn is_nested(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(table) if !table.contains_key("custom") => {
            looks_like_style(table) || table.values().any(is_nested)
        }
        _ => false,
    }
}

/// Looks for the node at the given dot-separated path.
///
/// Returns `None` if a component is missing, or if one that is not the last holds a value
/// rather than a namespace.
fn find_node<'a, N: Node>(map: &'a HashMap<String, N>, path: &str) -> Option<&'a N> {
    let mut map = map;
    let mut path = path;
    loop {
        match path.split_once('.') {
            None => return map.get(path),
            Some((namespace, rest)) => {
                map = map.get(namespace)?.namespace()?;
                path = rest;
            }
        }
    }
}

/// Returns the namespace with the given name, creating it if necessary.
///
/// A value with that name is replaced by an empty namespace.
fn namespace_entry<'a, N: Node>(
    map: &'a mut HashMap<String, N>,
    name: &str,
) -> &'a mut HashMap<String, N> {
    let entry = map.entry(name.to_string()).or_insert_with(N::new_namespace);
    if entry.namespace_mut().is_none() {
        *entry = N::new_namespace();
    }
    entry
        .namespace_mut()
        .expect("the entry was just made a namespace")
}

/// Inserts a node at the given dot-separated path, creating namespaces along the way.
///
/// A value on the way is replaced by a namespace; the node replaces whatever was at the end.
fn insert_node<N: Node>(map: &mut HashMap<String, N>, path: &str, node: N) {
    match path.split_once('.') {
        None => {
            map.insert(path.to_string(), node);
        }
        Some((namespace, rest)) => insert_node(namespace_entry(map, namespace), rest, node),
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
    /// The number of variants in this enum.
    pub const LENGTH: usize = crate::style::enum_length::<Self>();

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
    /// The number of variants in this enum.
    pub const LENGTH: usize = crate::style::enum_length::<Self>();

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

#[cfg(all(test, feature = "toml"))]
mod toml_style_tests {
    use super::*;
    use crate::style::{BaseColor, Color, ColorType};
    use std::sync::{Mutex, Once};

    static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

    struct Capture;

    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            if record.level() == log::Level::Warn {
                WARNINGS.lock().unwrap().push(record.args().to_string());
            }
        }

        fn flush(&self) {}
    }

    /// Loads the styles, and returns the palette with the warnings that mention `marker`.
    ///
    /// Tests run in parallel and share the logger: each uses its own names.
    fn load(marker: &str, toml: &str) -> (Palette, Vec<String>) {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let _ = log::set_logger(&Capture);
            log::set_max_level(log::LevelFilter::Warn);
        });
        let table: toml::value::Table = toml::from_str(toml).unwrap();
        let mut palette = Palette::retro();
        palette.load_toml_styles(&table);
        let warnings = WARNINGS
            .lock()
            .unwrap()
            .iter()
            .filter(|w| w.contains(marker))
            .cloned()
            .collect();
        (palette, warnings)
    }

    #[test]
    fn a_misspelled_key_is_one_warning_about_the_style() {
        let (palette, warnings) = load(
            "ts_typo",
            r#"
            [ts_typo]
            effects = ["bold"]
            front = "red"
            bakc = "blue"
            "#,
        );
        assert_eq!(palette.custom_style("ts_typo"), None);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("`ts_typo`"), "{warnings:?}");
        // Nothing was defined as a namespace member.
        assert_eq!(palette.custom_style("ts_typo.effects"), None);
        assert_eq!(palette.custom_style("ts_typo.front"), None);
    }

    #[test]
    fn a_wrong_value_type_is_one_warning_about_the_style() {
        let (palette, warnings) = load(
            "ts_type",
            r#"
            [ts_type]
            effects = ["bold"]
            front = 3
            back = "blue"
            "#,
        );
        assert_eq!(palette.custom_style("ts_type"), None);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("`ts_type`"), "{warnings:?}");
    }

    #[test]
    fn an_unknown_color_is_one_warning_about_the_style() {
        let (palette, warnings) = load(
            "ts_color",
            r#"
            [ts_color]
            effects = ["bold"]
            front = "rde"
            back = "blue"
            "#,
        );
        assert_eq!(palette.custom_style("ts_color"), None);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("`ts_color`"), "{warnings:?}");
    }

    #[test]
    fn a_partial_style_is_one_warning_too() {
        let (palette, warnings) = load(
            "ts_partial",
            r#"
            [ts_partial]
            front = "red"
            "#,
        );
        assert_eq!(palette.custom_style("ts_partial"), None);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("`ts_partial`"), "{warnings:?}");
    }

    #[test]
    fn a_standard_style_that_does_not_parse_is_one_warning() {
        let (_, warnings) = load(
            "title_primary",
            r#"
            [title_primary]
            effects = ["bold"]
            front = "rde"
            back = "blue"
            "#,
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }

    #[test]
    fn namespaces_are_still_namespaces() {
        let (palette, warnings) = load(
            "ts_ns",
            r##"
            [ts_ns.title]
            effects = ["bold"]
            front = "red"
            back = "blue"

            [ts_ns.deeper.still]
            effects = []
            front = { custom = "ts_ns.color" }
            back = "inherit_parent"
            "##,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(palette.custom_style("ts_ns.title").is_some());
        assert!(palette.custom_style("ts_ns.deeper.still").is_some());
        assert_eq!(palette.custom_style("ts_ns"), None);
    }

    #[test]
    fn a_namespace_can_hold_styles_named_like_style_keys() {
        let (palette, warnings) = load(
            "ts_keys",
            r##"
            [ts_keys.front]
            effects = ["bold"]
            front = "red"
            back = "blue"

            [ts_keys.effects.nested]
            effects = []
            front = "green"
            back = "inherit_parent"

            [ts_keys.back.deep.er]
            effects = []
            front = "green"
            back = "inherit_parent"
            "##,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        let front = palette.custom_style("ts_keys.front").unwrap();
        assert_eq!(
            front.color.front,
            ColorType::Color(Color::Dark(BaseColor::Red))
        );
        assert!(palette.custom_style("ts_keys.effects.nested").is_some());
        assert!(palette.custom_style("ts_keys.back.deep.er").is_some());
    }

    #[test]
    fn a_style_with_custom_colors_is_still_a_style() {
        let (palette, warnings) = load(
            "ts_cust_ok",
            r##"
            [ts_cust_ok]
            effects = []
            front = { custom = { name = "ts_cust_ok.c", fallback = "red" } }
            back = { custom = "ts_cust_ok.d" }
            "##,
        );
        assert!(palette.custom_style("ts_cust_ok").is_some());
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_malformed_color_table_is_a_malformed_style_not_a_namespace() {
        let (palette, warnings) = load(
            "ts_cb_one",
            r##"
            [ts_cb_one]
            effects = []
            front = { custm = "ts_cb_one.c" }
            back = "inherit_parent"
            "##,
        );
        assert_eq!(palette.custom_style("ts_cb_one"), None);
        assert_eq!(palette.custom_style("ts_cb_one.front"), None);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("`ts_cb_one`"), "{warnings:?}");

        let (palette, warnings) = load(
            "ts_cb_two",
            r##"
            [ts_cb_two]
            effects = []
            front = { custm = { name = "ts_cb_two.c" } }
            back = "inherit_parent"
            "##,
        );
        assert_eq!(palette.custom_style("ts_cb_two"), None);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("`ts_cb_two`"), "{warnings:?}");
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;
    use crate::style::{BaseColor, Effect};

    fn bold() -> Style {
        Style::from(Effect::Bold)
    }

    fn italic() -> Style {
        Style::from(Effect::Italic)
    }

    #[test]
    fn nested_and_dotted_names_are_the_same() {
        let mut palette = Palette::retro();
        palette.set_style("tabs.active", bold());
        palette.set_style("tabs.deeper.still", italic());
        palette.set_style("flat", bold());

        assert_eq!(palette.custom_style("tabs.active"), Some(&bold()));
        assert_eq!(palette.custom_style("tabs.deeper.still"), Some(&italic()));
        assert_eq!(palette.custom_style("flat"), Some(&bold()));

        // Namespaces are not styles, and cannot be walked through a style.
        assert_eq!(palette.custom_style("tabs"), None);
        assert_eq!(palette.custom_style("tabs.deeper"), None);
        assert_eq!(palette.custom_style("tabs.active.more"), None);
        assert_eq!(palette.custom_style("tabs.missing"), None);
        assert_eq!(palette.custom_style("nothing.active"), None);

        // The tree has the expected shape.
        let Some(StyleNode::Namespace(tabs)) = palette.custom_styles.get("tabs") else {
            panic!("`tabs` should be a namespace");
        };
        assert_eq!(tabs.get("active"), Some(&StyleNode::Style(bold())));

        // Another style in the same namespace keeps the first one.
        palette.set_style("tabs.inactive", italic());
        assert_eq!(palette.custom_style("tabs.active"), Some(&bold()));
        assert_eq!(palette.custom_style("tabs.inactive"), Some(&italic()));
    }

    #[test]
    fn standard_style_names_are_standard_only_as_a_whole() {
        let mut palette = Palette::retro();
        palette.set_style("highlight", italic());
        palette.set_style("tabs.highlight", bold());
        assert_eq!(palette[PaletteStyle::Highlight], italic());
        assert_eq!(palette.custom_style("highlight"), None);
        assert_eq!(palette.custom_style("tabs.highlight"), Some(&bold()));
    }

    #[test]
    fn style_keys_use_the_same_tree() {
        let key = StyleKey::from_palette("keyed.header", PaletteStyle::Primary);
        let mut palette = Palette::retro();
        palette.set_custom_style(&key, bold());
        assert_eq!(palette.custom_style("keyed.header"), Some(&bold()));
        palette.set_style("keyed.other", italic());
        assert_eq!(palette.custom_style(key.name()), Some(&bold()));
    }

    #[test]
    fn colors_and_styles_are_independent_trees() {
        let mut palette = Palette::retro();
        let red = Color::Dark(BaseColor::Red);
        palette.set_color("tabs.active", red);
        palette.set_style("tabs.active", bold());
        assert_eq!(palette.custom("tabs.active"), Some(&red));
        assert_eq!(palette.custom_style("tabs.active"), Some(&bold()));

        // A leaf in one tree does not touch the other.
        palette.set_color("tabs", red);
        assert_eq!(palette.custom("tabs"), Some(&red));
        assert_eq!(palette.custom_style("tabs.active"), Some(&bold()));
    }

    #[test]
    fn a_style_on_the_path_becomes_a_namespace() {
        let mut palette = Palette::retro();
        palette.set_style("tabs", bold());
        assert_eq!(palette.custom_style("tabs"), Some(&bold()));

        // Like `set_color`: the leaf is replaced by a namespace.
        palette.set_style("tabs.active", italic());
        assert_eq!(palette.custom_style("tabs"), None);
        assert_eq!(palette.custom_style("tabs.active"), Some(&italic()));

        // And a leaf replaces a namespace.
        palette.set_style("tabs", bold());
        assert_eq!(palette.custom_style("tabs"), Some(&bold()));
        assert_eq!(palette.custom_style("tabs.active"), None);

        // The same for colors.
        let red = Color::Dark(BaseColor::Red);
        palette.set_color("tabs", red);
        palette.set_color("tabs.active", red);
        assert_eq!(palette.custom("tabs"), None);
        assert_eq!(palette.custom("tabs.active"), Some(&red));
        palette.set_color("tabs", red);
        assert_eq!(palette.custom("tabs.active"), None);
    }

    #[test]
    fn merge_carries_flat_styles() {
        let mut palette = Palette::retro();
        palette.set_style("night.flat", bold());
        palette.set_style("flat", italic());
        palette.set_style("untouched", italic());

        let night = palette.merge("night");
        assert_eq!(night.custom_style("flat"), Some(&bold()));
        assert_eq!(night.custom_style("untouched"), Some(&italic()));
        // The namespace is still there, and the original is unchanged.
        assert_eq!(night.custom_style("night.flat"), Some(&bold()));
        assert_eq!(palette.custom_style("flat"), Some(&italic()));
    }

    #[test]
    fn merge_carries_nested_styles() {
        let mut palette = Palette::retro();
        palette.set_style("tabs.active", italic());
        palette.set_style("tabs.inactive", italic());
        palette.set_style("night.tabs.active", bold());
        palette.set_style("night.tabs.deeper.still", bold());

        let night = palette.merge("night");
        assert_eq!(night.custom_style("tabs.active"), Some(&bold()));
        assert_eq!(night.custom_style("tabs.deeper.still"), Some(&bold()));
        // The nested namespace replaces the one of the same name.
        assert_eq!(night.custom_style("tabs.inactive"), None);
        assert_eq!(palette.custom_style("tabs.active"), Some(&italic()));
        assert_eq!(palette.custom_style("tabs.inactive"), Some(&italic()));
    }

    #[test]
    fn merge_sets_standard_styles() {
        let mut palette = Palette::retro();
        let before = palette[PaletteStyle::Primary];
        palette.set_style("night.highlight", italic());
        palette.set_style("night.tabs.highlight", bold());

        let night = palette.merge("night");
        assert_eq!(night[PaletteStyle::Highlight], italic());
        assert_eq!(night[PaletteStyle::Primary], before);
        assert_eq!(night.custom_style("highlight"), None);
        assert_eq!(night.custom_style("tabs.highlight"), Some(&bold()));
        assert_ne!(palette[PaletteStyle::Highlight], italic());
    }

    #[test]
    fn merge_carries_colors_and_styles_together() {
        let mut palette = Palette::retro();
        let red = Color::Dark(BaseColor::Red);
        palette.set_color("night.view", red);
        palette.set_color("night.chart.axis", red);
        palette.set_color("night.mine", red);
        palette.set_style("night.mine", bold());
        palette.set_style("night.chart.title", bold());

        let night = palette.merge("night");
        assert_eq!(night[PaletteColor::View], red);
        assert_eq!(night.custom("chart.axis"), Some(&red));
        assert_eq!(night.custom("mine"), Some(&red));
        assert_eq!(night.custom_style("mine"), Some(&bold()));
        assert_eq!(night.custom_style("chart.title"), Some(&bold()));
        // Each tree only gets its own entries.
        assert_eq!(night.custom("chart.title"), None);
        assert_eq!(night.custom_style("chart.axis"), None);

        // A style namespace alone, or a color namespace alone, can be merged.
        let mut styles_only = Palette::retro();
        styles_only.set_style("dusk.mine", bold());
        assert_eq!(
            styles_only.merge("dusk").custom_style("mine"),
            Some(&bold())
        );
        let mut colors_only = Palette::retro();
        colors_only.set_color("dusk.mine", red);
        assert_eq!(colors_only.merge("dusk").custom("mine"), Some(&red));

        // Merging something that is not a namespace does nothing.
        let mut leaf = Palette::retro();
        leaf.set_style("dusk", bold());
        assert_eq!(leaf.merge("dusk"), leaf);
    }
}

#[cfg(all(test, feature = "toml"))]
mod toml_tree_tests {
    use super::*;
    use crate::style::{BaseColor, Effect};

    fn load(toml: &str) -> Palette {
        crate::theme::load_toml(toml).unwrap().palette
    }

    fn bold_red() -> Style {
        Style::from(Color::Dark(BaseColor::Red)).combine(Effect::Bold)
    }

    #[test]
    fn nested_tables_land_in_the_tree() {
        let palette = load(
            r#"
            [styles.tt_a.b]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"

            [styles.tt_a.c.d]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"
            "#,
        );
        assert_eq!(palette.custom_style("tt_a.b"), Some(&bold_red()));
        assert_eq!(palette.custom_style("tt_a.c.d"), Some(&bold_red()));
        assert_eq!(palette.custom_style("tt_a"), None);

        let Some(StyleNode::Namespace(a)) = palette.custom_styles.get("tt_a") else {
            panic!("`tt_a` should be a namespace");
        };
        assert!(matches!(a.get("b"), Some(StyleNode::Style(_))));
        assert!(matches!(a.get("c"), Some(StyleNode::Namespace(_))));
        // No flattened key.
        assert!(!palette.custom_styles.contains_key("tt_a.b"));
    }

    #[test]
    fn inline_tables_land_in_the_tree() {
        let palette = load(
            r#"
            [styles.tt_inline]
            active = { effects = ["bold"], front = "red", back = "inherit_parent" }
            deeper = { more = { effects = ["bold"], front = "red", back = "inherit_parent" } }

            [styles]
            tt_top = { effects = ["bold"], front = "red", back = "inherit_parent" }
            "#,
        );
        assert_eq!(palette.custom_style("tt_inline.active"), Some(&bold_red()));
        assert_eq!(
            palette.custom_style("tt_inline.deeper.more"),
            Some(&bold_red())
        );
        assert_eq!(palette.custom_style("tt_top"), Some(&bold_red()));
    }

    #[test]
    fn both_notations_give_the_same_palette() {
        let nested = load(
            r#"
            [styles.tt_n.active]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"
            "#,
        );
        let inline = load(
            r#"
            [styles.tt_n]
            active = { effects = ["bold"], front = "red", back = "inherit_parent" }
            "#,
        );
        assert_eq!(nested, inline);
    }

    #[test]
    fn styles_and_colors_share_namespace_names() {
        let palette = load(
            r##"
            [colors.tt_both]
            active = "#010203"

            [styles.tt_both.active]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"
            "##,
        );
        assert_eq!(palette.custom("tt_both.active"), Some(&Color::Rgb(1, 2, 3)));
        assert_eq!(palette.custom_style("tt_both.active"), Some(&bold_red()));
    }

    #[test]
    fn loading_replaces_the_namespaces_it_defines() {
        let mut theme = crate::theme::Theme::retro();
        theme.palette.set_color("tt_ns.old", Color::Rgb(1, 1, 1));
        theme
            .palette
            .set_style("tt_ns.old", Style::from(Effect::Italic));
        theme.palette.set_color("tt_keep.old", Color::Rgb(1, 1, 1));
        theme
            .palette
            .set_style("tt_keep.old", Style::from(Effect::Italic));
        let table: toml::value::Table = toml::from_str(
            r##"
            [colors.tt_ns]
            new = "#020202"
            [styles.tt_ns.new]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"
            "##,
        )
        .unwrap();
        theme.load_toml(&table);

        // The namespace is replaced, in both trees.
        assert_eq!(theme.palette.custom("tt_ns.old"), None);
        assert_eq!(theme.palette.custom_style("tt_ns.old"), None);
        assert_eq!(
            theme.palette.custom("tt_ns.new"),
            Some(&Color::Rgb(2, 2, 2))
        );
        assert_eq!(theme.palette.custom_style("tt_ns.new"), Some(&bold_red()));

        // A namespace the file does not define is left alone.
        assert!(theme.palette.custom("tt_keep.old").is_some());
        assert!(theme.palette.custom_style("tt_keep.old").is_some());
    }

    #[test]
    fn loading_sets_single_entries_without_replacing_their_namespace() {
        let mut theme = crate::theme::Theme::retro();
        theme.palette.set_color("tt_one.old", Color::Rgb(1, 1, 1));
        theme
            .palette
            .set_style("tt_one.old", Style::from(Effect::Italic));
        let table: toml::value::Table = toml::from_str(
            r##"
            [colors]
            "tt_one.new" = "#020202"
            [styles]
            "tt_one.new" = { effects = ["bold"], front = "red", back = "inherit_parent" }
            "##,
        )
        .unwrap();
        theme.load_toml(&table);
        assert!(theme.palette.custom("tt_one.old").is_some());
        assert!(theme.palette.custom("tt_one.new").is_some());
        assert!(theme.palette.custom_style("tt_one.old").is_some());
        assert!(theme.palette.custom_style("tt_one.new").is_some());
    }

    #[test]
    fn quoted_dotted_keys_in_a_namespace_are_namespace_boundaries() {
        let palette = load(
            r##"
            [colors.tt_dot]
            "a.b" = "#010203"
            [colors.tt_dot.deep]
            "x.y" = "#040506"

            [styles.tt_dot."a.b"]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"
            [styles.tt_dot]
            "c.d" = { effects = ["bold"], front = "red", back = "inherit_parent" }
            [styles.tt_dot.deep]
            "x.y" = { effects = ["bold"], front = "red", back = "inherit_parent" }
            [styles."tt_top.sub"."p.q"]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"
            "##,
        );
        assert_eq!(palette.custom("tt_dot.a.b"), Some(&Color::Rgb(1, 2, 3)));
        assert_eq!(
            palette.custom("tt_dot.deep.x.y"),
            Some(&Color::Rgb(4, 5, 6))
        );
        assert_eq!(palette.custom_style("tt_dot.a.b"), Some(&bold_red()));
        assert_eq!(palette.custom_style("tt_dot.c.d"), Some(&bold_red()));
        assert_eq!(palette.custom_style("tt_dot.deep.x.y"), Some(&bold_red()));
        assert_eq!(palette.custom_style("tt_top.sub.p.q"), Some(&bold_red()));
        // No literal key with a dot in it.
        assert_eq!(palette, {
            let mut expected = load("");
            expected.set_color("tt_dot.a.b", Color::Rgb(1, 2, 3));
            expected.set_color("tt_dot.deep.x.y", Color::Rgb(4, 5, 6));
            for key in [
                "tt_dot.a.b",
                "tt_dot.c.d",
                "tt_dot.deep.x.y",
                "tt_top.sub.p.q",
            ] {
                expected.set_style(key, bold_red());
            }
            expected
        });
    }

    #[test]
    fn empty_tables_define_nothing() {
        let palette = load(
            r##"
            [colors.tt_empty]
            [colors.tt_nested.inner]
            [colors."tt.dotted"]
            [styles.tt_empty]
            [styles.tt_nested.inner]
            [styles."tt.dotted"]
            "##,
        );
        assert_eq!(palette, load(""));
    }

    #[test]
    fn an_empty_table_does_not_clear_a_namespace() {
        let mut theme = crate::theme::Theme::retro();
        theme.palette.set_color("tt_kept.old", Color::Rgb(1, 1, 1));
        theme.palette.set_style("tt_kept.old", bold_red());
        let table: toml::value::Table =
            toml::from_str("[colors.tt_kept]\n[styles.tt_kept]").unwrap();
        theme.load_toml(&table);
        assert!(theme.palette.custom("tt_kept.old").is_some());
        assert!(theme.palette.custom_style("tt_kept.old").is_some());
    }

    #[test]
    fn loaded_namespaces_can_be_merged() {
        let palette = load(
            r#"
            [styles.tt_night.tabs.active]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"

            [styles.tt_night.highlight]
            effects = ["bold"]
            front = "red"
            back = "inherit_parent"
            "#,
        );
        let night = palette.merge("tt_night");
        assert_eq!(night.custom_style("tabs.active"), Some(&bold_red()));
        assert_eq!(night[PaletteStyle::Highlight], bold_red());
    }
}

#[cfg(all(test, feature = "toml", feature = "builder"))]
mod notation_tests {
    use super::*;
    use crate::builder::Context;
    use crate::style::{BaseColor, ColorStyle, ColorType, Effect, PaletteColor};
    use crate::theme::Theme;
    use serde_json::json;

    /// The theme used by these tests, written with Rust calls.
    fn from_rust() -> Palette {
        let mut bold = Effects::empty();
        bold.insert(Effect::Bold);
        let title = Style {
            effects: bold,
            color: ColorStyle::new(
                ColorType::Custom(ColorKey::intern("chart.axis", ColorType::InheritParent)),
                ColorType::InheritParent,
            ),
        };
        let mut palette = Palette::default();
        palette[PaletteColor::View] = Color::Rgb(1, 2, 3);
        palette.set_color("chart.axis", Color::Rgb(0x55, 0x55, 0x55));
        palette.set_color("chart.deep.grid", Color::Dark(BaseColor::Red));
        palette.set_style("highlight", title);
        palette.set_style("chart.title", title);
        palette.set_style("chart.deep.mark", title);
        palette
    }

    #[test]
    fn rust_toml_and_blueprint_build_the_same_palette() {
        let toml = crate::theme::load_toml(
            r##"
            [colors]
            view = "#010203"
            [colors.chart]
            axis = "#555555"
            [colors.chart.deep]
            grid = "red"

            [styles]
            highlight = { effects = ["bold"], front = { custom = "chart.axis" }, back = "inherit_parent" }
            [styles.chart]
            title = { effects = ["bold"], front = { custom = "chart.axis" }, back = "inherit_parent" }
            [styles.chart.deep.mark]
            effects = ["bold"]
            front = { custom = "chart.axis" }
            back = "inherit_parent"
            "##,
        )
        .unwrap();

        let title = json!({
            "effects": ["bold"],
            "front": {"custom": "chart.axis"},
            "back": "inherit_parent",
        });
        let blueprint = Context::new()
            .resolve::<Theme>(&json!({
                "palette": {
                    "view": "#010203",
                    "chart": {"axis": "#555555", "deep": {"grid": "red"}},
                },
                "styles": {
                    "highlight": title,
                    "chart": {"title": title, "deep": {"mark": title}},
                },
            }))
            .unwrap();

        assert_eq!(toml.palette, from_rust());
        assert_eq!(blueprint.palette, from_rust());
    }

    #[test]
    fn empty_tables_are_the_same_in_toml_and_blueprints() {
        // An empty table of colors defines nothing, in both notations. (An empty table of styles
        // is a style in a blueprint, see `resolve_palette_styles`: it is not written here.)
        let toml = crate::theme::load_toml("[colors.empty]\n[styles.empty]").unwrap();
        let blueprint = Context::new()
            .resolve::<Theme>(&json!({"palette": {"empty": {}}}))
            .unwrap();
        assert_eq!(toml.palette, blueprint.palette);
    }
}
