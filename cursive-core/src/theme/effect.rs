use enumset::EnumSetType;
use std::str::FromStr;

/// Text effect
#[allow(clippy::derive_hash_xor_eq)] // We do derive it through EnumSetType
#[derive(EnumSetType, Debug, Hash)]
pub enum Effect {
    /// No effect
    Simple,
    /// Reverses foreground and background colors
    Reverse,
    /// Prints foreground as "dim" or "faint" (has no effect for ncurses/pancurses/blt backends)
    Dim,
    /// Prints foreground in bold
    Bold,
    /// Prints foreground in italic
    Italic,
    /// Prints foreground with strikethrough (has no effect for ncurses and blt backends)
    Strikethrough,
    /// Prints foreground with underline
    Underline,
    /// Foreground text blinks (background color is static).
    Blink,
}

impl FromStr for Effect {
    type Err = super::NoSuchColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Simple" | "simple" => Effect::Simple,
            "Reverse" | "reverse" => Effect::Reverse,
            "Dim" | "dim" => Effect::Dim,
            "Bold" | "bold" => Effect::Bold,
            "Italic" | "italic" => Effect::Italic,
            "Strikethrough" | "strikethrough" => Effect::Strikethrough,
            "Underline" | "underline" => Effect::Underline,
            "Blink" | "blink" => Effect::Blink,
            _ => return Err(super::NoSuchColor),
        })
    }
}
