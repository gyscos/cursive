use crate::style::PaletteStyle;
use crate::theme::Theme;
use crate::view::{View, ViewWrapper};
use parking_lot::Mutex;

type Patch = dyn Fn(&mut Theme) + Send + Sync;

/// Adjusts the current theme for the wrapped view.
///
/// Unlike [`ThemedView`](crate::views::ThemedView), which replaces the entire theme, this starts
/// from the theme this view is drawn with (from the application, or from a parent view), and
/// applies some changes to it. The result follows any later change to that parent theme.
///
/// This is the way for a view to set the custom colors and styles used by its children.
///
/// If the overlay changes what a view looks like (the `View` style), the wrapped view is drawn
/// with the new one, but only when the overlay is itself drawn with the `View` style of the parent
/// theme. In any other style (highlighted, a title...), that style is kept.
///
/// # Examples
///
/// ```rust
/// use cursive_core::style::{BaseColor, ColorKey, ColorStyle, PaletteColor};
/// use cursive_core::utils::markup::StyledString;
/// use cursive_core::views::{TextView, ThemeOverlay};
///
/// // A view (possibly from another crate) uses a custom color...
/// static WARNING: ColorKey = ColorKey::new("my_crate.warning", BaseColor::Yellow.dark());
/// let text = TextView::new(StyledString::styled(
///     "Careful!",
///     ColorStyle::front(&WARNING),
/// ));
///
/// // ... and here, that color is going to be red.
/// let view = ThemeOverlay::new(text, |theme| {
///     theme
///         .palette
///         .set_custom_color(&WARNING, BaseColor::Red.light());
///     // Standard colors can be changed too, possibly based on the parent theme.
///     theme.palette[PaletteColor::View] = theme.palette[PaletteColor::Background];
/// });
/// ```
///
/// # Blueprint
///
/// In a blueprint, the `theme_overlay` decorator can set the `shadow`, `borders`, `palette`
/// and `styles` of the theme. Colors in `palette` are concrete colors, or `{parent: name}` to
/// take the color `name` from the parent theme: a standard palette color (`background`) or a
/// custom color (`charts.axis`). Names are looked up in the theme as it was before this
/// overlay changed anything, so entries do not depend on each other or on their order. If the
/// parent theme does not define the name, a warning is logged and the entry is left unchanged.
///
/// As a consequence, a namespace here cannot have `parent` as its only color: it would be read
/// as a reference to the parent theme.
///
/// ```yaml
/// - theme_overlay:
///     palette:
///       # Use the background color of the parent theme for views.
///       view: { parent: background }
///       # Namespaces work like in a theme.
///       my_crate:
///         accent: { parent: title_primary }
///         axis: { parent: charts.axis }
///         # Concrete colors can be mixed in.
///         warning: light red
///     styles:
///       # Standard styles, or custom ones.
///       highlight: underline
///       # Namespaces of styles work like the ones of colors: this is `my_crate.title`.
///       my_crate:
///         title: [bold]
///       # Dotted names are the same.
///       my_crate.caption: [italic]
/// ```
///
/// Colors and styles nest the same way. In `styles`, an object is a style if it has one of the
/// keys of a style (`effects`, `color`, `front`, `back`), or if it can be read as one
/// (`{bold: on}`, `{custom: name}`, `{}`); any other object is a namespace. A namespace can hold
/// styles called `effects`, `front` or `back` as long as they are objects holding a style or
/// another namespace. A misspelled key in a style object is an error (`{front: red, bakc: blue}`)
/// but `{bakc: blue}` alone is a namespace holding a style called `bakc`.
pub struct ThemeOverlay<T> {
    view: T,
    patch: Box<Patch>,
    // The last theme we were drawn with, and the result of patching it.
    cache: Mutex<Option<(Theme, Theme)>>,
}

impl<T> ThemeOverlay<T> {
    /// Wraps the given view.
    ///
    /// `patch` is given a copy of the theme this view is drawn with, and should apply the
    /// desired changes. It is only called again when that theme changes, so it should always
    /// give the same result for the same input.
    pub fn new<F>(view: T, patch: F) -> Self
    where
        F: Fn(&mut Theme) + Send + Sync + 'static,
    {
        ThemeOverlay {
            view,
            patch: Box::new(patch),
            cache: Mutex::new(None),
        }
    }

    /// Replaces the changes applied to the theme.
    pub fn set_patch<F>(&mut self, patch: F)
    where
        F: Fn(&mut Theme) + Send + Sync + 'static,
    {
        self.patch = Box::new(patch);
        *self.cache.get_mut() = None;
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for ThemeOverlay<T> {
    wrap_impl!(self.view: T; same_size);

    fn wrap_draw(&self, printer: &crate::Printer) {
        let parent = printer.theme;

        let mut cache = self.cache.lock();
        if cache.as_ref().is_none_or(|(cached, _)| cached != parent) {
            let mut theme = parent.clone();
            (self.patch)(&mut theme);
            *cache = Some((parent.clone(), theme));
        }
        let (_, theme) = cache.as_ref().unwrap();

        // The printer already resolved the current style using the parent theme. If we are drawn
        // in an ordinary view context (the current style is the view style of the parent theme)
        // and the patch changed what a view looks like by default, we need to apply that again.
        // Otherwise we leave the current style alone: we could be drawn in a place with a
        // different style on purpose (highlighted, a title...), and this must not be lost.
        let current = printer.current_style();
        let view_style = |theme: &Theme| {
            PaletteStyle::View
                .resolve(&theme.palette)
                .resolve(&theme.palette, current)
        };
        let restyle = view_style(parent) == current && view_style(theme) != current;

        let printer = printer.theme(theme);
        if restyle {
            printer.with_style(PaletteStyle::View, |printer| self.view.draw(printer));
        } else {
            self.view.draw(&printer);
        }
    }
}

/// Returns a color from a palette, by the name of a standard palette color or a custom color.
#[cfg(feature = "builder")]
fn parent_color(palette: &crate::style::Palette, name: &str) -> Option<crate::style::Color> {
    match name.parse::<crate::style::PaletteColor>() {
        Ok(color) => Some(palette[color]),
        Err(_) => palette.custom(name).copied(),
    }
}

crate::manual_blueprint!(with theme_overlay, |config, context| {
    use crate::builder::resolvable::{OverlayColor, resolve_overlay_colors, resolve_palette_styles};

    let shadow: Option<bool> = context.resolve(&config["shadow"])?;
    let borders: Option<crate::style::BorderStyle> = context.resolve(&config["borders"])?;
    let colors = resolve_overlay_colors(&config["palette"], context)?;
    let styles = resolve_palette_styles(&config["styles"], context)?;

    Ok(move |view| {
        ThemeOverlay::new(view, move |theme| {
            if let Some(shadow) = shadow {
                theme.shadow = shadow;
            }
            if let Some(borders) = borders {
                theme.borders = borders;
            }
            // Read the colors from the parent theme before changing anything.
            let colors: Vec<_> = colors
                .iter()
                .map(|(key, color)| {
                    let color = match color {
                        OverlayColor::Color(color) => Some(*color),
                        OverlayColor::Parent(name) => {
                            let parent = parent_color(&theme.palette, name);
                            if parent.is_none() {
                                log::warn!(
                                    "Theme overlay: `{key}` refers to `{name}`, \
                                     which the parent theme does not define."
                                );
                            }
                            parent
                        }
                    };
                    (key, color)
                })
                .collect();
            for (key, color) in colors {
                if let Some(color) = color {
                    theme.palette.set_color(key, color);
                }
            }
            for (key, style) in &styles {
                theme.palette.set_style(key, *style);
            }
        })
    })
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::style::{BaseColor, Color, ColorKey, ColorType, PaletteColor};
    use crate::{Printer, Vec2};
    use parking_lot::RwLock;
    use std::sync::Arc;

    static KEY: ColorKey = ColorKey::from_palette("overlay_test.key", PaletteColor::Secondary);

    /// Records the colors it would be drawn with.
    struct Probe(Arc<Mutex<Vec<(Color, Color)>>>);

    impl View for Probe {
        fn draw(&self, printer: &Printer) {
            let current = printer.current_color();
            let key = ColorType::from(&KEY).resolve(&printer.theme.palette, current.front);
            self.0.lock().push((key, current.back));
        }
    }

    /// Draws the view like the application does: in the view style of the given theme.
    fn draw(view: &impl View, theme: &Theme) {
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(10, 10));
        let buffer = RwLock::new(buffer);
        let printer = Printer::new((10, 10), theme, &buffer);
        printer.with_style(PaletteStyle::View, |printer| view.draw(printer));
    }

    #[test]
    fn overlay_follows_parent_theme() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let red = Color::Light(BaseColor::Red);

        // Without any overlay, the key uses its fallback.
        let mut theme = Theme::retro();
        let probe = Probe(seen.clone());
        draw(&probe, &theme);

        // This overlay sets the custom color, and changes the view color based on the parent.
        let overlay = ThemeOverlay::new(probe, move |theme| {
            theme.palette.set_custom_color(&KEY, red);
            theme.palette[PaletteColor::View] = theme.palette[PaletteColor::Background];
        });
        draw(&overlay, &theme);

        // Changing the parent theme is picked up by the overlay.
        theme.palette[PaletteColor::Background] = Color::Rgb(1, 2, 3);
        draw(&overlay, &theme);

        let retro = Theme::retro().palette;
        assert_eq!(
            *seen.lock(),
            vec![
                (retro[PaletteColor::Secondary], retro[PaletteColor::View]),
                (red, retro[PaletteColor::Background]),
                (red, Color::Rgb(1, 2, 3)),
            ]
        );
    }

    #[test]
    fn overlay_keeps_current_style_when_view_style_is_unchanged() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let overlay = ThemeOverlay::new(Probe(seen.clone()), |theme| {
            theme.palette.set_custom_color(&KEY, Color::Rgb(7, 7, 7));
        });

        let theme = Theme::retro();
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(10, 10));
        let buffer = RwLock::new(buffer);
        let printer = Printer::new((10, 10), &theme, &buffer);
        printer.with_style(PaletteStyle::Highlight, |printer| overlay.draw(printer));

        // The background is still the one from the highlight style, not the view color.
        let highlight = printer_style(&theme, PaletteStyle::Highlight);
        assert_eq!(*seen.lock(), vec![(Color::Rgb(7, 7, 7), highlight)]);
    }

    #[test]
    fn overlay_keeps_a_deliberate_style_when_only_the_view_color_changes() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        // The overlay only changes the view color.
        let overlay = ThemeOverlay::new(Probe(seen.clone()), |theme| {
            theme.palette[PaletteColor::View] = Color::Rgb(1, 2, 3);
        });

        let theme = Theme::retro();
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(10, 10));
        let buffer = RwLock::new(buffer);
        let printer = Printer::new((10, 10), &theme, &buffer);
        // The parent draws us in a highlighted place, on purpose.
        printer.with_style(PaletteStyle::Highlight, |printer| overlay.draw(printer));

        let highlight = printer_style(&theme, PaletteStyle::Highlight);
        let (_, back) = seen.lock()[0];
        assert_eq!(back, highlight, "the overlay replaced the highlight style");
    }

    #[test]
    fn overlay_around_a_layer_gets_its_own_view_color() {
        use crate::views::{DummyView, Layer};

        // Like the showcase dashboard: views are drawn on the background color.
        let overlay = ThemeOverlay::new(Layer::new(DummyView), |theme| {
            theme.palette[PaletteColor::View] = theme.palette[PaletteColor::Background];
        });

        let theme = Theme::retro();
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(10, 10));
        let buffer = RwLock::new(buffer);
        let printer = Printer::new((10, 10), &theme, &buffer);
        printer.with_style(PaletteStyle::View, |printer| overlay.draw(printer));

        let style = buffer.read().cell_style(Vec2::new(3, 4)).unwrap();
        assert_eq!(style.color.back, theme.palette[PaletteColor::Background]);
        assert_ne!(style.color.back, theme.palette[PaletteColor::View]);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn overlay_from_config() {
        let config = r#"
            TextView:
                content: Hello
                with:
                    - theme_overlay:
                        borders: none
                        palette:
                            view: black
                            overlay_test:
                                key: light red
                        styles:
                            overlay_test.title: bold
        "#;
        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();

        let overlay = view
            .downcast_ref::<ThemeOverlay<crate::views::BoxedView>>()
            .expect("the top-level view should be the overlay");

        let mut theme = Theme::retro();
        (overlay.patch)(&mut theme);
        let palette = &theme.palette;
        assert_eq!(theme.borders, crate::style::BorderStyle::None);
        assert_eq!(palette[PaletteColor::View], Color::Dark(BaseColor::Black));
        assert_eq!(
            palette.custom(KEY.name()),
            Some(&Color::Light(BaseColor::Red))
        );
        assert_eq!(
            palette.custom_style("overlay_test.title"),
            Some(&crate::style::Style::from(crate::style::Effect::Bold))
        );
    }

    #[cfg(feature = "builder")]
    #[test]
    fn overlay_styles_nest_like_colors() {
        use crate::style::{Effect, Effects, Style, StyleKey, StyleType};

        static TITLE: StyleKey =
            StyleKey::from_palette("overlay_nest.title", PaletteStyle::Primary);
        static DEEP: StyleKey =
            StyleKey::from_palette("overlay_nest.deep.er", PaletteStyle::Primary);

        let patch = |styles: &str| {
            let config = format!(
                "TextView:\n  content: Hello\n  with:\n    - theme_overlay:\n        styles:\n{}",
                styles
                    .lines()
                    .map(|line| format!("          {line}\n"))
                    .collect::<String>()
            );
            let config: crate::builder::Config = yaml_serde::from_str(&config).unwrap();
            let view = crate::builder::Context::new().build(&config).unwrap();
            let mut theme = Theme::retro();
            let overlay = view
                .downcast_ref::<ThemeOverlay<crate::views::BoxedView>>()
                .expect("the top-level view should be the overlay");
            (overlay.patch)(&mut theme);
            theme
        };

        let nested = patch(
            "overlay_nest:\n  title: [bold]\n  deep:\n    er: [italic]\nhighlight: [underline]",
        );
        let dotted = patch(
            "overlay_nest.title: [bold]\noverlay_nest.deep.er: [italic]\nhighlight: [underline]",
        );
        assert_eq!(nested, dotted);

        let palette = &nested.palette;
        let bold = Style::from(Effects::only(Effect::Bold));
        assert_eq!(palette.custom_style("overlay_nest.title"), Some(&bold));
        assert_eq!(
            palette.custom_style("overlay_nest.deep.er"),
            Some(&Style::from(Effects::only(Effect::Italic)))
        );
        assert_eq!(
            palette[PaletteStyle::Highlight],
            Style::from(Effects::only(Effect::Underline))
        );
        // Style keys read them.
        assert_eq!(StyleType::from(&TITLE).resolve(palette), bold);
        assert_eq!(
            StyleType::from(&DEEP).resolve(palette),
            Style::from(Effects::only(Effect::Italic))
        );
    }

    #[cfg(feature = "builder")]
    fn overlay_from_palette(palette: &str) -> Box<dyn Fn(&mut Theme) + Send + Sync> {
        let config = format!(
            "TextView:\n  content: Hello\n  with:\n    - theme_overlay:\n        palette:\n{}",
            palette
                .lines()
                .map(|line| format!("          {line}\n"))
                .collect::<String>()
        );
        let config: crate::builder::Config = yaml_serde::from_str(&config).unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        assert!(
            view.downcast_ref::<ThemeOverlay<crate::views::BoxedView>>()
                .is_some(),
            "the top-level view should be the overlay"
        );
        Box::new(move |theme| {
            let overlay = view.downcast_ref::<ThemeOverlay<crate::views::BoxedView>>();
            (overlay.unwrap().patch)(theme)
        })
    }

    #[cfg(feature = "builder")]
    #[test]
    fn overlay_from_config_parent_color() {
        let patch = overlay_from_palette("view: { parent: background }");

        let mut theme = Theme::retro();
        patch(&mut theme);
        let retro = Theme::retro().palette;
        assert_eq!(
            theme.palette[PaletteColor::View],
            retro[PaletteColor::Background]
        );

        // Changing the parent theme is picked up through the cache.
        let seen = Arc::new(Mutex::new(Vec::new()));
        let overlay = ThemeOverlay::new(Probe(seen.clone()), move |theme| patch(theme));
        let mut theme = Theme::retro();
        draw(&overlay, &theme);
        theme.palette[PaletteColor::Background] = Color::Rgb(1, 2, 3);
        draw(&overlay, &theme);
        let secondary = retro[PaletteColor::Secondary];
        assert_eq!(
            *seen.lock(),
            vec![
                (secondary, retro[PaletteColor::Background]),
                (secondary, Color::Rgb(1, 2, 3)),
            ]
        );
    }

    #[cfg(feature = "builder")]
    #[test]
    fn overlay_from_config_parent_custom() {
        // Custom source (dotted), standard target, and custom target.
        let patch = overlay_from_palette(
            "view: { parent: charts.axis }\nmine:\n  copy: { parent: charts.axis }",
        );

        let mut theme = Theme::retro();
        theme.palette.set_color("charts.axis", Color::Rgb(4, 5, 6));
        patch(&mut theme);
        assert_eq!(theme.palette[PaletteColor::View], Color::Rgb(4, 5, 6));
        assert_eq!(
            theme.palette.custom("mine.copy"),
            Some(&Color::Rgb(4, 5, 6))
        );
        assert_eq!(
            theme.palette.custom("charts.axis"),
            Some(&Color::Rgb(4, 5, 6))
        );
    }

    #[cfg(feature = "builder")]
    #[test]
    fn overlay_from_config_parent_snapshot() {
        // Two entries swap their values: both read the parent theme.
        let patch = overlay_from_palette(
            "view: { parent: background }\nbackground: { parent: view }\nmine:\n  a: { parent: mine.b }\n  b: { parent: mine.a }",
        );

        let mut theme = Theme::retro();
        let (view, background) = (
            theme.palette[PaletteColor::View],
            theme.palette[PaletteColor::Background],
        );
        theme.palette.set_color("mine.a", Color::Rgb(1, 1, 1));
        theme.palette.set_color("mine.b", Color::Rgb(2, 2, 2));
        patch(&mut theme);
        assert_eq!(theme.palette[PaletteColor::View], background);
        assert_eq!(theme.palette[PaletteColor::Background], view);
        assert_eq!(theme.palette.custom("mine.a"), Some(&Color::Rgb(2, 2, 2)));
        assert_eq!(theme.palette.custom("mine.b"), Some(&Color::Rgb(1, 1, 1)));
    }

    #[cfg(feature = "builder")]
    #[test]
    fn overlay_from_config_parent_unknown() {
        let patch = overlay_from_palette(
            "view: { parent: no_such_color }\nmine:\n  a: { parent: no.such.entry }\n  b: { parent: charts }\nsecondary: light red",
        );

        let mut theme = Theme::retro();
        theme.palette.set_color("charts.axis", Color::Rgb(4, 5, 6));
        theme.palette.set_color("mine.a", Color::Rgb(1, 1, 1));
        let before = theme.clone();
        patch(&mut theme);

        // Unknown names (or namespaces) leave the entry unchanged.
        assert_eq!(
            theme.palette[PaletteColor::View],
            before.palette[PaletteColor::View]
        );
        assert_eq!(theme.palette.custom("mine.a"), Some(&Color::Rgb(1, 1, 1)));
        assert_eq!(theme.palette.custom("mine.b"), None);
        // Other entries still apply.
        assert_eq!(
            theme.palette[PaletteColor::Secondary],
            Color::Light(BaseColor::Red)
        );
    }

    #[cfg(feature = "builder")]
    #[test]
    fn overlay_from_config_parent_mixed() {
        let patch = overlay_from_palette(
            "view: black\nbackground: { parent: view }\nmine:\n  a: { parent: primary }\n  b: light red",
        );

        let mut theme = Theme::retro();
        let (view, primary) = (
            theme.palette[PaletteColor::View],
            theme.palette[PaletteColor::Primary],
        );
        patch(&mut theme);
        assert_eq!(
            theme.palette[PaletteColor::View],
            Color::Dark(BaseColor::Black)
        );
        // The parent value is read before `view` is changed.
        assert_eq!(theme.palette[PaletteColor::Background], view);
        assert_eq!(theme.palette.custom("mine.a"), Some(&primary));
        assert_eq!(
            theme.palette.custom("mine.b"),
            Some(&Color::Light(BaseColor::Red))
        );
    }

    fn printer_style(theme: &Theme, style: PaletteStyle) -> Color {
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(1, 1));
        let buffer = RwLock::new(buffer);
        let printer = Printer::new((1, 1), theme, &buffer);
        let mut back = None;
        printer.with_style(style, |printer| back = Some(printer.current_color().back));
        back.unwrap()
    }
}
