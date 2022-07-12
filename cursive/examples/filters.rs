use cursive::view::Resizable;
use cursive::views::{Dialog, LinearLayout, ListView, RadioGroup};
use cursive::With;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Color {
    Red,
    Blue,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Size {
    Large,
    Small,
}

#[derive(Clone, Copy, Debug, Default)]
struct Filters {
    color: Option<Color>,
    size: Option<Size>,
}

// This is an example with a more complex use of user_data.
// Here we prepare some state (Filters) and make it available to be used
// elsewhere via user_data.

fn main() {
    let mut siv = cursive::default();
    siv.set_user_data(Filters::default());
    // Leave the menu visible
    siv.set_autohide_menu(false);
    siv.menubar()
        .add_leaf("Filters", |s| {
            // Hide menu while Filters are open to prevent other Filter windows
            // to be created on top.
            s.set_autohide_menu(true);

            let mut color_group: RadioGroup<Option<Color>> = RadioGroup::new();
            let mut size_group: RadioGroup<Option<Size>> = RadioGroup::new();

            // Get current filters to draw the correct selected buttons.
            let current_filters = s
                .with_user_data(|filters: &mut Filters| filters.clone())
                .unwrap();
            s.add_layer(
                Dialog::new()
                    .title("Filters")
                    .content(
                        ListView::new()
                            .child(
                                "Color: ",
                                LinearLayout::horizontal()
                                    .child(
                                        color_group
                                            .button(None, "Any")
                                            .fixed_width(10),
                                    )
                                    .child(
                                        color_group
                                            .button(Some(Color::Red), "Red")
                                            .with_if(
                                                current_filters.color
                                                    == Some(Color::Red),
                                                |button| {
                                                    button.select();
                                                },
                                            )
                                            .fixed_width(10),
                                    )
                                    .child(
                                        color_group
                                            .button(Some(Color::Blue), "Blue")
                                            .with_if(
                                                current_filters.color
                                                    == Some(Color::Blue),
                                                |button| {
                                                    button.select();
                                                },
                                            ),
                                    ),
                            )
                            .child(
                                "Size: ",
                                LinearLayout::horizontal()
                                    .child(
                                        size_group
                                            .button(None, "Any")
                                            .fixed_width(10),
                                    )
                                    .child(
                                        size_group
                                            .button(Some(Size::Small), "Small")
                                            .with_if(
                                                current_filters.size
                                                    == Some(Size::Small),
                                                |button| {
                                                    button.select();
                                                },
                                            )
                                            .fixed_width(10),
                                    )
                                    .child(
                                        size_group
                                            .button(Some(Size::Large), "Large")
                                            .with_if(
                                                current_filters.size
                                                    == Some(Size::Large),
                                                |button| {
                                                    button.select();
                                                },
                                            ),
                                    ),
                            ),
                    )
                    .button("Done", move |s| {
                        // Save selected filters as user_data
                        s.with_user_data(|filters: &mut Filters| {
                            filters.color = *color_group.selection();
                            filters.size = *size_group.selection();
                        })
                        .unwrap();
                        // Bring back the menu. Enable opening Filter window
                        // again once this one is closed.
                        s.set_autohide_menu(false);
                        s.pop_layer();
                    }),
            );
        })
        .add_delimiter();

    siv.add_layer(
        Dialog::text("Some stuff happens here.")
            .title("Main")
            .button("Quit", |s| s.quit()),
    );

    siv.run();
}
