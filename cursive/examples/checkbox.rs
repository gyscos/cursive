use std::{cell::RefCell, collections::HashSet, fmt::Display, rc::Rc};

use cursive::views::{Checkbox, Dialog, DummyView, LinearLayout};

// This example uses checkboxes.
#[derive(Debug, PartialEq, Eq, Hash)]
enum Toppings {
    ChocolateSprinkles,
    CrushedAlmonds,
    StrawberrySauce,
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

fn main() {
    let mut siv = cursive::default();

    // TODO: placeholder for MultiChoiceGroup.

    // Application wide container w/toppings choices.
    let toppings: Rc<RefCell<HashSet<Toppings>>> = Rc::new(RefCell::new(HashSet::new()));

    siv.add_layer(
        Dialog::new()
            .title("Make your selections")
            .content(
                LinearLayout::vertical()
                    .child(Checkbox::labelled("Chocolate Sprinkles".into()).on_change({
                        let toppings = toppings.clone();
                        move |_, checked| {
                            if checked {
                                toppings.borrow_mut().insert(Toppings::ChocolateSprinkles);
                            } else {
                                toppings.borrow_mut().remove(&Toppings::ChocolateSprinkles);
                            }
                        }
                    }))
                    .child(Checkbox::labelled("Crushed Almonds".into()).on_change({
                        let toppings = toppings.clone();
                        move |_, checked| {
                            if checked {
                                toppings.borrow_mut().insert(Toppings::CrushedAlmonds);
                            } else {
                                toppings.borrow_mut().remove(&Toppings::CrushedAlmonds);
                            }
                        }
                    }))
                    .child(Checkbox::labelled("Strawberry Sauce".into()).on_change({
                        let toppings = toppings.clone();
                        move |_, checked| {
                            if checked {
                                toppings.borrow_mut().insert(Toppings::StrawberrySauce);
                            } else {
                                toppings.borrow_mut().remove(&Toppings::StrawberrySauce);
                            }
                        }
                    })),
            )
            .button("Ok", move |s| {
                s.pop_layer();
                let toppings = toppings
                    .borrow()
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                let text = format!("Toppings: {toppings}");
                s.add_layer(Dialog::text(text).button("Ok", |s| s.quit()));
            }),
    );

    siv.run();
}
