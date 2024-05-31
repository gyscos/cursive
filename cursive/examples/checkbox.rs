use ahash::HashSet;
use cursive::views::{Checkbox, Dialog, DummyView, LinearLayout, MultiChoiceGroup};
use parking_lot::Mutex;
use std::fmt::Display;
use std::sync::Arc;

// This example uses checkboxes.
#[derive(Debug, PartialEq, Eq, Hash)]
enum Toppings {
    ChocolateSprinkles,
    CrushedAlmonds,
    StrawberrySauce,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Extras {
    Tissues,
    DarkCone,
    ChocolateFlake,
}

impl Display for Toppings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Toppings::ChocolateSprinkles => write!(f, "Chocolate Sprinkles"),
            Toppings::CrushedAlmonds => write!(f, "Crushed Almonds"),
            Toppings::StrawberrySauce => write!(f, "Strawberry Sauce"),
        }
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
    let toppings: Arc<Mutex<HashSet<Toppings>>> = Arc::new(Mutex::new(HashSet::default()));

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
                                    if checked {
                                        toppings.lock().insert(Toppings::ChocolateSprinkles);
                                    } else {
                                        toppings.lock().remove(&Toppings::ChocolateSprinkles);
                                    }
                                }
                            }))
                            .child(Checkbox::labelled("Crushed Almonds").on_change({
                                let toppings = Arc::clone(&toppings);
                                move |_, checked| {
                                    if checked {
                                        toppings.lock().insert(Toppings::CrushedAlmonds);
                                    } else {
                                        toppings.lock().remove(&Toppings::CrushedAlmonds);
                                    }
                                }
                            }))
                            .child(Checkbox::labelled("Strawberry Sauce").on_change({
                                let toppings = Arc::clone(&toppings);
                                move |_, checked| {
                                    if checked {
                                        toppings.lock().insert(Toppings::StrawberrySauce);
                                    } else {
                                        toppings.lock().remove(&Toppings::StrawberrySauce);
                                    }
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
                let toppings = toppings
                    .lock()
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
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
