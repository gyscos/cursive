//! This example demonstrates how to use a multi-choice group with checkboxes to
//! allow users to select multiple values in a set.
use cursive::views::{Checkbox, Dialog, DummyView, LinearLayout, MultiChoiceGroup};
use parking_lot::Mutex;
use std::fmt::Display;
use std::sync::Arc;

/// This example uses standalone checkboxes.
#[derive(Debug, Default, PartialEq, Eq, Hash)]
struct Toppings {
    chocolate_sprinkles: bool,
    crushed_almonds: bool,
    strawberry_sauce: bool,
}

/// This example uses checkboxes from the multi-choice group.
#[derive(Debug, PartialEq, Eq, Hash)]
enum Extras {
    Tissues,
    DarkCone,
    ChocolateFlake,
}

impl Display for Toppings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.chocolate_sprinkles {
            write!(f, "Chocolate Sprinkles")?;
        }
        if self.chocolate_sprinkles && self.crushed_almonds {
            write!(f, ", ")?;
        }
        if self.crushed_almonds {
            write!(f, "Crushed Almonds")?;
        }
        if self.crushed_almonds && self.strawberry_sauce {
            write!(f, ", ")?;
        }
        if self.strawberry_sauce {
            write!(f, "Strawberry Sauce")?;
        }
        Ok(())
    }
}

impl Display for Extras {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Extras::Tissues => write!(f, "Tissues"),
            Extras::DarkCone => write!(f, "Dark Cone"),
            Extras::ChocolateFlake => write!(f, "Chocolate Flake"),
        }
    }
}

fn main() {
    let mut siv = cursive::default();

    // Application wide container w/toppings choices.
    let toppings: Arc<Mutex<Toppings>> = Arc::new(Mutex::new(Toppings::default()));

    // The `MultiChoiceGroup<T>` can be used to maintain multiple choices.
    let mut multichoice: MultiChoiceGroup<Extras> = MultiChoiceGroup::new();

    siv.add_layer(
        Dialog::new()
            .title("Make your selections")
            .content(
                LinearLayout::horizontal()
                    .child(
                        LinearLayout::vertical()
                            .child(Checkbox::labelled("Chocolate Sprinkles").on_change({
                                let toppings = Arc::clone(&toppings);
                                move |_, checked| {
                                    toppings.lock().chocolate_sprinkles = checked;
                                }
                            }))
                            .child(Checkbox::labelled("Crushed Almonds").on_change({
                                let toppings = Arc::clone(&toppings);
                                move |_, checked| {
                                    toppings.lock().crushed_almonds = checked;
                                }
                            }))
                            .child(Checkbox::labelled("Strawberry Sauce").on_change({
                                let toppings = Arc::clone(&toppings);
                                move |_, checked| {
                                    toppings.lock().strawberry_sauce = checked;
                                }
                            })),
                    )
                    .child(DummyView)
                    .child(
                        LinearLayout::vertical()
                            .child(multichoice.checkbox(Extras::ChocolateFlake, "Chocolate Flake"))
                            .child(multichoice.checkbox(Extras::DarkCone, "Dark Cone"))
                            .child(multichoice.checkbox(Extras::Tissues, "Tissues")),
                    ),
            )
            .button("Ok", move |s| {
                s.pop_layer();
                let toppings = toppings.lock().to_string();
                let extras = multichoice
                    .selections()
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                let text = format!("Toppings: {toppings}\nExtras: {extras}");
                s.add_layer(Dialog::text(text).button("Ok", |s| s.quit()));
            }),
    );

    siv.run();
}
