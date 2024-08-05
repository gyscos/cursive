use enum_map::{Enum, EnumMap};
use enumset::{EnumSet, EnumSetType};
use std::str::FromStr;

/// A concrete set of effects to enable.
///
/// Every missing effect should be disabled.
pub type ConcreteEffects = EnumSet<Effect>;

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

impl Effect {
    /// Returns the order of the effect in the effect set/map.
    ///
    /// This is very brittle and should be kept in sync with the enum definition. Might benefit
    /// from a proc macro.
    ///
    /// This is all because enum_map's Enum derive is trait-based and does not support const fn.
    pub(crate) const fn ordinal(self) -> usize {
        match self {
            Effect::Simple => 0,
            Effect::Reverse => 1,
            Effect::Dim => 2,
            Effect::Bold => 3,
            Effect::Italic => 4,
            Effect::Strikethrough => 5,
            Effect::Underline => 6,
            Effect::Blink => 7,
        }
    }
}

/// A set of effects status.
///
/// Describes what to do for each effect: enable, disable, preserve, xor.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Effects {
    /// The status of each effect.
    pub statuses: EnumMap<Effect, EffectStatus>,
}

impl Default for Effects {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<ConcreteEffects> for Effects {
    fn from(other: ConcreteEffects) -> Self {
        let mut result = Self::default();
        for effect in other {
            result.statuses[effect] = EffectStatus::OppositeParent;
        }
        result
    }
}

impl Effects {
    /// An empty set of effects.
    pub const EMPTY: Self = Effects::empty();

    /// Return an empty set of effects.
    ///
    /// They will all be set to `InheritParent`.
    pub const fn empty() -> Self {
        let statuses = [EffectStatus::InheritParent; Effect::LENGTH];
        Self {
            statuses: EnumMap::from_array(statuses),
        }
    }

    /// Sets the given effect to be `InheritParent`.
    pub fn remove(&mut self, effect: Effect) {
        self.statuses[effect] = EffectStatus::InheritParent;
    }

    /// Sets the given effect to be `OppositeParent`.
    pub fn insert(&mut self, effect: Effect) {
        self.statuses[effect] = EffectStatus::OppositeParent;
    }

    /// Helper function to implement `Self::only()`.
    const fn status_for(i: usize, effect: Effect) -> EffectStatus {
        if i == effect.ordinal() {
            EffectStatus::OppositeParent
        } else {
            EffectStatus::InheritParent
        }
    }

    /// Return a set of effects with only one effect.
    ///
    /// It will be set to `OppositeParent`. Every other effect will be `InheritParent`.
    pub const fn only(effect: Effect) -> Self {
        // TODO: make this less brittle?
        let statuses = [
            Self::status_for(0, effect),
            Self::status_for(1, effect),
            Self::status_for(2, effect),
            Self::status_for(3, effect),
            Self::status_for(4, effect),
            Self::status_for(5, effect),
            Self::status_for(6, effect),
            Self::status_for(7, effect),
        ];

        Self {
            statuses: EnumMap::from_array(statuses),
        }
    }

    /// Resolve an effects directive into concrete effects.
    pub fn resolve(&self, old: ConcreteEffects) -> ConcreteEffects {
        let mut result = ConcreteEffects::default();
        for (effect, status) in self.statuses {
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

    /// Merge the two sets of effects.
    pub fn merge(mut old: Self, new: Self) -> Self {
        for (effect, status) in new.statuses {
            old.statuses[effect] = EffectStatus::merge(old.statuses[effect], status);
        }
        old
    }
}

impl std::ops::Index<Effect> for Effects {
    type Output = EffectStatus;

    fn index(&self, index: Effect) -> &Self::Output {
        &self.statuses[index]
    }
}

impl std::ops::IndexMut<Effect> for Effects {
    fn index_mut(&mut self, index: Effect) -> &mut Self::Output {
        &mut self.statuses[index]
    }
}

/// Describes what to do with an effect.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum EffectStatus {
    /// Force the effect on, regardless of the parent.
    On,

    /// Force the effect off, regardless of the parent.
    Off,

    /// Keep the same effect status as the parent.
    InheritParent,

    /// Use the opposite state from the parent (XOR).
    OppositeParent,
}

impl EffectStatus {
    /// Returns the opposite status.
    ///
    /// * Swaps `On` and `Off`.
    /// * Swaps `InheritParent` and `OppositeParent`.
    pub const fn swap(self) -> Self {
        match self {
            EffectStatus::On => EffectStatus::Off,
            EffectStatus::Off => EffectStatus::On,
            EffectStatus::InheritParent => EffectStatus::OppositeParent,
            EffectStatus::OppositeParent => EffectStatus::InheritParent,
        }
    }

    /// Merges the old status with the new one.
    pub const fn merge(old: Self, new: Self) -> Self {
        match new {
            EffectStatus::On => EffectStatus::On,
            EffectStatus::Off => EffectStatus::Off,
            EffectStatus::InheritParent => old,
            EffectStatus::OppositeParent => old.swap(),
        }
    }
}
impl FromStr for EffectStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "On" | "on" | "true" => Self::On,
            "Off" | "off" | "false" => Self::Off,
            "InheritParent" | "inherit_parent" => Self::InheritParent,
            "OppositeParent" | "opposite_parent" => Self::OppositeParent,
            _ => return Err(()),
        })
    }
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
