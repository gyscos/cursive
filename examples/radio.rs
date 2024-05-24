use cursive::views::{Dialog, DummyView, LinearLayout, RadioButton, RadioGroup};

// This example uses radio buttons.
#[derive(Debug)]
enum Flavor {
    Vanilla,
    Strawberry,
    Chocolate,
}

fn main() {
    let mut siv = cursive::default();

    // We need to pre-create the groups for our RadioButtons.
    let mut color_group: RadioGroup<String> = RadioGroup::new();
    let mut size_group: RadioGroup<u32> = RadioGroup::new();
    // The last group will be global

    siv.add_layer(
        Dialog::new()
            .title("Make your selection")
            // We'll have two columns side-by-side
            .content(
                LinearLayout::horizontal()
                    .child(
                        LinearLayout::vertical()
                            // The color group uses the label itself as stored value
                            // By default, the first item is selected.
                            .child(color_group.button_str("Red"))
                            .child(color_group.button_str("Green"))
                            .child(color_group.button_str("Blue")),
                    )
                    // A DummyView is used as a spacer
                    .child(DummyView)
                    .child(
                        LinearLayout::vertical()
                            // For the size, we store a number separately
                            .child(size_group.button(5, "Small"))
                            // The initial selection can also be overridden
                            .child(size_group.button(15, "Medium").selected())
                            // The large size is out of stock, sorry!
                            .child(size_group.button(25, "Large").disabled()),
                    )
                    .child(DummyView)
                    .child(LinearLayout::vertical()
                        .child(RadioButton::global("flavor", Flavor::Vanilla, "Vanilla"))
                        .child(RadioButton::global("flavor", Flavor::Strawberry, "Strawberry"))
                        .child(RadioButton::global("flavor", Flavor::Chocolate, "Chocolate"))
                    )
            )
            .button("Ok", move |s| {
                // We retrieve the stored value for both group.
                let color = color_group.selection();
                let size = size_group.selection();
                let flavor = RadioGroup::<Flavor>::with_global("flavor", |group| group.selection());

                s.pop_layer();
                // And we simply print the result.
                let text = format!("Color: {color}\nSize: {size}cm\nFlavor: {flavor:?}");
                s.add_layer(Dialog::text(text).button("Ok", |s| s.quit()));
            }),
    );

    siv.run();
}
