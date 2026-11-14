use crate::direction::Orientation;

use enum_map::Enum;

use std::str::FromStr;

/// Specifies which glyphs the standard views use for their decorations.
///
/// Form widgets draw small decorations: the mark in a checkbox, the brackets around a button,
/// the filler of an edit view, the track of a slider or a scrollbar... This selects the
/// characters used for them.
///
/// [`GlyphSet::Ascii`] is the default, and only uses plain ASCII characters (except for the
/// scrollbar thumb). [`GlyphSet::Unicode`] uses nicer characters that most modern terminals
/// and fonts can display.
///
/// Views cannot read the theme when computing their size, so **every glyph slot has the same
/// display width in every set**: switching sets never changes a layout.
///
/// Explicit per-view settings (like [`SelectView::set_decorators`] or [`EditView::set_filler`])
/// always win over the theme.
///
/// It can be set with `glyphs = "unicode"` in a TOML theme, or in the `glyphs` field of
/// [`Theme`](crate::theme::Theme).
///
/// ```rust
/// use cursive_core::style::GlyphSet;
///
/// let mut theme = cursive_core::theme::Theme::default();
/// assert_eq!(theme.glyphs, GlyphSet::Ascii);
/// theme.glyphs = GlyphSet::Unicode;
/// ```
///
/// [`SelectView::set_decorators`]: crate::views::SelectView::set_decorators
/// [`EditView::set_filler`]: crate::views::EditView::set_filler
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Enum)]
pub enum GlyphSet {
    /// Plain ASCII glyphs: `[X]`, `(X)`, `<Button>`, `_`... This is the default.
    #[default]
    Ascii,
    /// Unicode glyphs: `[✓]`, `(●)`, `‹Button›`, `·`...
    Unicode,
}

impl GlyphSet {
    /// The number of variants in this enum.
    pub const LENGTH: usize = crate::style::enum_length::<Self>();

    /// Returns an iterator on all possible glyph sets.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }

    /// Mark drawn inside a checked checkbox.
    pub(crate) fn checkbox_mark(self) -> &'static str {
        match self {
            GlyphSet::Ascii => "X",
            GlyphSet::Unicode => "✓",
        }
    }

    /// Mark drawn inside a selected radio button.
    pub(crate) fn radio_mark(self) -> &'static str {
        match self {
            GlyphSet::Ascii => "X",
            GlyphSet::Unicode => "●",
        }
    }

    /// Left and right brackets around a button label.
    pub(crate) fn button_brackets(self) -> [&'static str; 2] {
        match self {
            GlyphSet::Ascii => ["<", ">"],
            GlyphSet::Unicode => ["‹", "›"],
        }
    }

    /// Decorators around the selected item of a popup `SelectView`.
    pub(crate) fn select_decorators(self) -> [&'static str; 2] {
        match self {
            GlyphSet::Ascii => ["<", ">"],
            GlyphSet::Unicode => [" ", "▾"],
        }
    }

    /// Filler for the empty space in text inputs.
    pub(crate) fn edit_filler(self) -> &'static str {
        match self {
            GlyphSet::Ascii => "_",
            GlyphSet::Unicode => "·",
        }
    }

    /// Track of a slider in the given orientation.
    pub(crate) fn slider_track(self, orientation: Orientation) -> &'static str {
        match (self, orientation) {
            (GlyphSet::Ascii, Orientation::Horizontal) => "-",
            (GlyphSet::Ascii, Orientation::Vertical) => "|",
            (GlyphSet::Unicode, Orientation::Horizontal) => "─",
            (GlyphSet::Unicode, Orientation::Vertical) => "│",
        }
    }

    /// Track of a scrollbar in the given orientation.
    pub(crate) fn scrollbar_track(self, orientation: Orientation) -> &'static str {
        self.slider_track(orientation)
    }

    /// Thumb of a scrollbar.
    pub(crate) fn scrollbar_thumb(self) -> &'static str {
        match self {
            GlyphSet::Ascii => "▒",
            GlyphSet::Unicode => "█",
        }
    }
}

impl FromStr for GlyphSet {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ascii" => Ok(GlyphSet::Ascii),
            "unicode" => Ok(GlyphSet::Unicode),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn parse_glyph_sets() {
        assert_eq!("ascii".parse(), Ok(GlyphSet::Ascii));
        assert_eq!("unicode".parse(), Ok(GlyphSet::Unicode));
        assert_eq!("fancy".parse::<GlyphSet>(), Err(()));
        for set in GlyphSet::all() {
            let name = format!("{set:?}").to_lowercase();
            assert_eq!(name.parse(), Ok(set));
        }
        assert_eq!(GlyphSet::default(), GlyphSet::Ascii);
    }

    /// Every slot must have the same width in every set.
    #[test]
    fn same_widths() {
        let slots = |set: GlyphSet| -> Vec<&'static str> {
            let mut v = vec![
                set.checkbox_mark(),
                set.radio_mark(),
                set.edit_filler(),
                set.scrollbar_thumb(),
            ];
            v.extend(set.button_brackets());
            v.extend(set.select_decorators());
            for o in [Orientation::Horizontal, Orientation::Vertical] {
                v.push(set.slider_track(o));
                v.push(set.scrollbar_track(o));
            }
            v
        };
        let reference = slots(GlyphSet::Ascii);
        for set in GlyphSet::all() {
            let glyphs = slots(set);
            assert_eq!(glyphs.len(), reference.len());
            for (glyph, ascii) in glyphs.iter().zip(&reference) {
                assert_eq!(glyph.width(), ascii.width(), "{set:?}: {glyph:?}");
                assert_eq!(glyph.width(), 1, "{set:?}: {glyph:?}");
            }
        }
    }
}
