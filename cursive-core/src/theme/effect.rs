use enumset::EnumSetType;

/// Text effect
#[allow(clippy::derive_hash_xor_eq)] // We do derive it through EnumSetType
#[derive(EnumSetType, Debug, Hash)]
pub enum Effect {
    /// No effect
    Simple,
    /// Reverses foreground and background colors
    Reverse,
    /// Prints foreground in bold
    Bold,
    /// Prints foreground in italic
    Italic,
    /// Prints foreground with strikethrough (has no effect for ncurses and blt backends)
    Strikethrough,
    /// Prints foreground with underline
    Underline,
}
