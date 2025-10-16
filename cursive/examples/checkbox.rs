//! This example demonstrates how to use a checkboxes manually to
//! allow users to select multiple values in a set.
use ahash::HashSet;
use cursive::views::{Checkbox, Dialog, DummyView, LinearLayout};
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

// #[derive(Debug, PartialEq, Eq, Hash)]
// enum Extras {
//     Tissues,
//     DarkCone,
//     ChocolateFlake,
// }

#[derive(Debug, Default)]
struct Extras {
    tissues: bool,
    dark_cone: bool,
    chocolate_flake: bool,
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
        let extras = [
            if self.tissues { "Tissues" } else { "" },
            if self.dark_cone { "Dark Cone" } else { "" },
            if self.chocolate_flake {
                "Chocolate Flake"
            } else {
                ""
            },
        ];
        write!(
            f,
            "{}",
            extras
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join(", ")
        )
    }
}

fn main() {
    let mut siv = cursive::default();

    // Application wide container w/toppings choices.
    let toppings: Arc<Mutex<HashSet<Toppings>>> = Arc::new(Mutex::new(HashSet::default()));

    // Application wide container w/extras choices.
    let extras: Arc<Mutex<Extras>> = Arc::new(Mutex::new(Extras::default()));

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
                            .child(Checkbox::labelled("Chocolate Flake").on_change({
                                let extras = Arc::clone(&extras);
                                move |_, checked| {
                                    extras.lock().chocolate_flake = checked;
                                }
                            }))
                            .child(Checkbox::labelled("Dark Cone").on_change({
                                let extras = Arc::clone(&extras);
                                move |_, checked| {
                                    extras.lock().dark_cone = checked;
                                }
                            }))
                            .child(Checkbox::labelled("Tissues").on_change({
                                let extras = Arc::clone(&extras);
                                move |_, checked| {
                                    extras.lock().tissues = checked;
                                }
                            })),
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
                let extras = extras.lock().to_string();
                let text = format!("Toppings: {toppings}\nExtras: {extras}");
                s.add_layer(Dialog::text(text).button("Ok", |s| s.quit()));
            }),
    );

    siv.run();
}
