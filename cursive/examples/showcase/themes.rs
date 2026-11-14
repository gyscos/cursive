//! Color schemes for the dashboard.
//!
//! Each scheme is turned into a regular cursive `Theme`, so every standard
//! view (dialogs, menus, buttons, ...) follows along.
//!
//! The dashboard needs a few more colors than the standard palette offers
//! (accents, status colors): these are custom palette entries, defined by the
//! keys below. Views and styled text refer to the keys, and get whatever
//! color the current theme gives them.

use cursive::style::{
    BORDER, BORDER_FOCUSED, BaseColor, BorderStyle, Color, ColorKey, ColorStyle, Effect, GlyphSet,
    Palette, PaletteColor, Rgb, Style,
};
use cursive::theme::Theme;

// Each key comes with a fallback, used with themes that know nothing about
// this example.
pub static ACCENT: ColorKey = ColorKey::from_palette("showcase.accent", PaletteColor::TitlePrimary);
pub static ACCENT2: ColorKey =
    ColorKey::from_palette("showcase.accent2", PaletteColor::TitleSecondary);
pub static DIM: ColorKey = ColorKey::from_palette("showcase.dim", PaletteColor::Tertiary);
pub static SURFACE: ColorKey = ColorKey::from_palette("showcase.surface", PaletteColor::View);
pub static OK: ColorKey = ColorKey::new("showcase.ok", Color::Dark(BaseColor::Green));
pub static WARN: ColorKey = ColorKey::new("showcase.warn", Color::Dark(BaseColor::Yellow));
pub static ERR: ColorKey = ColorKey::new("showcase.err", Color::Dark(BaseColor::Red));

pub struct Scheme {
    pub name: &'static str,
    /// Background of the dashboard.
    pub bg: Rgb,
    /// Background of things floating above it: dialogs, menus.
    pub surface: Rgb,
    /// Regular text.
    pub fg: Rgb,
    /// De-emphasized text, borders, grid lines.
    pub dim: Rgb,
    pub accent: Rgb,
    pub accent2: Rgb,
    pub ok: Rgb,
    pub warn: Rgb,
    pub err: Rgb,
    pub shadow: Rgb,
}

const fn rgb(hex: u32) -> Rgb {
    Rgb::from_u32(hex)
}

pub const SCHEMES: &[Scheme] = &[
    Scheme {
        name: "Midnight",
        bg: rgb(0x0f111a),
        surface: rgb(0x1c2030),
        fg: rgb(0xc8d3f5),
        dim: rgb(0x4a5173),
        accent: rgb(0x4fd6be),
        accent2: rgb(0xc099ff),
        ok: rgb(0x9ece6a),
        warn: rgb(0xffc777),
        err: rgb(0xff5f7e),
        shadow: rgb(0x05060a),
    },
    Scheme {
        name: "Ember",
        bg: rgb(0x1d2021),
        surface: rgb(0x32302f),
        fg: rgb(0xebdbb2),
        dim: rgb(0x7c6f64),
        accent: rgb(0xfabd2f),
        accent2: rgb(0xfe8019),
        ok: rgb(0xb8bb26),
        warn: rgb(0xfabd2f),
        err: rgb(0xfb4934),
        shadow: rgb(0x101010),
    },
    Scheme {
        name: "Paper",
        bg: rgb(0xf4f1ea),
        surface: rgb(0xffffff),
        fg: rgb(0x2e3440),
        dim: rgb(0xa7adba),
        accent: rgb(0x3b6fd4),
        accent2: rgb(0xb0429c),
        ok: rgb(0x3f9142),
        warn: rgb(0xc27c0e),
        err: rgb(0xd03a3a),
        shadow: rgb(0xc9c4b8),
    },
    Scheme {
        name: "Synthwave",
        bg: rgb(0x1a1033),
        surface: rgb(0x2a1b4d),
        fg: rgb(0xf0e6ff),
        dim: rgb(0x6b5a9c),
        accent: rgb(0xff4fd8),
        accent2: rgb(0x36e0ff),
        ok: rgb(0x72f1b8),
        warn: rgb(0xfede5d),
        err: rgb(0xfe4450),
        shadow: rgb(0x0c0718),
    },
];

impl Scheme {
    /// Theme for the whole application.
    ///
    /// Views (dialogs, menus) sit on the `surface` color.
    pub fn theme(&self) -> Theme {
        use cursive::style::PaletteColor::*;
        use cursive::style::PaletteStyle;

        let mut palette = Palette::default();
        palette[Background] = self.bg.into();
        palette[Shadow] = self.shadow.into();
        palette[View] = self.surface.into();
        palette[Primary] = self.fg.into();
        palette[Secondary] = self.dim.into();
        palette[Tertiary] = self.dim.into();
        palette[TitlePrimary] = self.accent.into();
        palette[TitleSecondary] = self.accent2.into();
        palette[Highlight] = self.accent.into();
        palette[HighlightInactive] = self.dim.into();
        palette[HighlightText] = self.bg.into();

        // Dim borders for every box (panels, dialogs).
        palette.set_custom_color(&BORDER, self.dim.into());
        // ... except around what has the focus.
        palette.set_custom_color(&BORDER_FOCUSED, self.accent.into());

        // Custom colors, used by the dashboard.
        palette.set_custom_color(&ACCENT, self.accent.into());
        palette.set_custom_color(&ACCENT2, self.accent2.into());
        palette.set_custom_color(&DIM, self.dim.into());
        palette.set_custom_color(&SURFACE, self.surface.into());
        palette.set_custom_color(&OK, self.ok.into());
        palette.set_custom_color(&WARN, self.warn.into());
        palette.set_custom_color(&ERR, self.err.into());

        // Text fields: regular text on a slightly different background.
        let field = Style::from(ColorStyle::new(Color::from(self.fg), Color::from(self.bg)));
        palette[PaletteStyle::EditableText] = field;
        palette[PaletteStyle::EditableTextInactive] = field;
        palette[PaletteStyle::EditableTextCursor] = field.combine(Effect::Reverse);
        palette[PaletteStyle::TitlePrimary] =
            Style::from(ColorStyle::title_primary()).combine(Effect::Bold);

        Theme {
            shadow: true,
            borders: BorderStyle::Rounded,
            glyphs: GlyphSet::Unicode,
            palette,
        }
    }
}
