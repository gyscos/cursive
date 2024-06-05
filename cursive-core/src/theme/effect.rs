use enum_map::{Enum, EnumMap};
use enumset::{EnumSet, EnumSetType};
use std::str::FromStr;

/// Text effect
#[allow(clippy::derived_hash_with_manual_eq)] // We do derive it through EnumSetType
#[derive(EnumSetType, Enum, Debug, Hash)]
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

/// A set of effects status.
///
/// Describes what to do for each effect: enable, disable, preserve, xor.
pub struct Effects(pub EnumMap<Effect, EffectStatus>);

impl Effects {
    /// Resolve an effects directive into concrete effects.
    pub fn resolve(&self, old: ConcreteEffects) -> ConcreteEffects {
        let mut result = ConcreteEffects::default();
        for (effect, status) in self.0 {
            if matches!(
                (status, old.contains(effect)),
                (EffectStatus::On, _)
                    | (EffectStatus::InheritParent, true)
                    | (EffectStatus::OppositeParent, false)
            ) {
                result.insert(effect);
            }
        }
        result
    }
}

/// A concrete set of effects to enable.
///
/// Every missing effect should be disabled.
pub type ConcreteEffects = EnumSet<Effect>;

/// Describes what to do with an effect.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EffectStatus {
    /// Force the effect on, regardless of the parent.
    On,

    /// Force the effect off, regardless of the parent.
    Off,

    /// Keep the same effect status as the parent.
    InheritParent,

    /// Use the opposite state from the parent.
    OppositeParent,
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
