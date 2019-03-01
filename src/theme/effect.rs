use enumset::EnumSetType;

/// Text effect
#[derive(EnumSetType, Debug)]
pub enum Effect {
    /// No effect
    Simple,
    /// Reverses foreground and background colors
    Reverse,
    /// Prints foreground in bold
    Bold,
    /// Prints foreground in italic
    Italic,
    /// Prints foreground with underline
    Underline,
}
