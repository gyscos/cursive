use cursive::view::Resizable;
use cursive::views::{Dialog, LinearLayout, RadioGroup, TextView};

#[derive(Clone, Copy, Debug)]
enum Color {
    Red,
    Blue,
}

#[derive(Clone, Copy, Debug)]
enum Size {
    Large,
    Small,
}

#[derive(Default)]
struct Filters {
    color: Option<Color>,
    size: Option<Size>,
}

// This is an examples of a Settings/Configuration "pop-up" where we can
// modify user_data to be used elsewhere in our application.

fn main() {
    let mut siv = cursive::default();
    siv.set_user_data(Filters::default());
    // Leave the menu visible
    siv.set_autohide_menu(false);
    siv.menubar()
        .add_leaf("Filters", |s| {
            // Hide menu while "pop-up" settings are visible
            s.set_autohide_menu(true);

            let mut color_group: RadioGroup<Option<Color>> = RadioGroup::new();
            let mut size_group: RadioGroup<Option<Size>> = RadioGroup::new();

            // Get current filters to draw the correct selected buttons.
            let user_data = s.user_data::<Filters>().unwrap();
            let current_color = user_data.color;
            let current_size = user_data.size;
            s.add_layer(
                Dialog::new()
                    .title("Filters")
                    .content(
                        LinearLayout::vertical()
                            .child(
                                LinearLayout::horizontal()
                                    .child(
                                        TextView::new("Color:")
                                            .fixed_width(15)
                                            .fixed_height(2),
                                    )
                                    .child(
                                        color_group
                                            .button(None, "Any")
                                            .fixed_width(10),
                                    )
                                    .child({
                                        let mut button = color_group
                                            .button(Some(Color::Red), "Red");
                                        if let Some(Color::Red) = current_color
                                        {
                                            button.select();
                                        }
                                        button.fixed_width(10)
                                    })
                                    .child({
                                        let mut button = color_group
                                            .button(Some(Color::Blue), "Blue");
                                        if let Some(Color::Blue) =
                                            current_color
                                        {
                                            button.select();
                                        }
                                        button.fixed_width(10)
                                    }),
                            )
                            .child(
                                LinearLayout::horizontal()
                                    .child(
                                        TextView::new("Size:")
                                            .fixed_width(15)
                                            .fixed_height(2),
                                    )
                                    .child(
                                        size_group
                                            .button(None, "Any")
                                            .fixed_width(10),
                                    )
                                    .child({
                                        let mut button = size_group.button(
                                            Some(Size::Small),
                                            "Small",
                                        );
                                        if let Some(Size::Small) = current_size
                                        {
                                            button.select();
                                        }
                                        button.fixed_width(10)
                                    })
                                    .child({
                                        let mut button = size_group.button(
                                            Some(Size::Large),
                                            "Large",
                                        );
                                        if let Some(Size::Large) = current_size
                                        {
                                            button.select();
                                        }
                                        button.fixed_width(10)
                                    }),
                            ),
                    )
                    .button("Done", move |s| {
                        // Save selected filters as user_data
                        s.user_data::<Filters>().unwrap().color =
                            *color_group.selection();
                        s.user_data::<Filters>().unwrap().size =
                            *size_group.selection();
                        s.pop_layer();
                        s.set_autohide_menu(false);
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
