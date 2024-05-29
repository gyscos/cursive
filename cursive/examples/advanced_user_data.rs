use cursive::view::{Nameable, Resizable};
use cursive::views::{Dialog, EditView, LinearLayout, ListView, RadioGroup, SliderView};
use cursive::With;

#[derive(Clone, Debug, Default)]
struct UserData {
    boolean: bool,
    string: String,
    number: usize,
}

// This is an example with a more complex use of user_data.
// Here we prepare some state (UserData) and make it available to be used
// elsewhere via user_data.

fn main() {
    let mut siv = cursive::default();
    siv.set_user_data(UserData::default());

    siv.add_layer(
        Dialog::text("Some stuff happens here.")
            .title("Main")
            .button("UserData", |s| {
                let mut boolean_group: RadioGroup<bool> = RadioGroup::new();

                // Get current user_data to draw the correct current status.
                let current_data = s
                    .with_user_data(|user_data: &mut UserData| user_data.clone())
                    .unwrap();
                s.add_layer(
                    Dialog::new()
                        .title("UserData")
                        .content(
                            ListView::new()
                                .child(
                                    "String: ",
                                    EditView::new()
                                        .content(current_data.string.clone())
                                        .with_name("string")
                                        .fixed_width(18),
                                )
                                .child(
                                    "Number: ",
                                    SliderView::horizontal(18)
                                        .value(current_data.number)
                                        .with_name("number"),
                                )
                                .child(
                                    "Boolean: ",
                                    LinearLayout::horizontal()
                                        .child(boolean_group.button(false, "False").fixed_width(10))
                                        .child(
                                            boolean_group
                                                .button(true, "True")
                                                .with_if(current_data.boolean, |button| {
                                                    button.select();
                                                })
                                                .fixed_width(10),
                                        )
                                        .with(|layout| {
                                            if current_data.boolean {
                                                layout.set_focus_index(1).unwrap();
                                            }
                                        }),
                                ),
                        )
                        .button("Done", move |s| {
                            // Save selected user_data as user_data
                            let string = s
                                .call_on_name("string", |view: &mut EditView| view.get_content())
                                .unwrap();
                            let number = s
                                .call_on_name("number", |view: &mut SliderView| view.get_value())
                                .unwrap();
                            s.with_user_data(|user_data: &mut UserData| {
                                user_data.boolean = *boolean_group.selection();
                                user_data.string = string.to_string();
                                user_data.number = number;
                            })
                            .unwrap();
                            s.pop_layer();
                        }),
                );
            }),
    );

    siv.run();
}
