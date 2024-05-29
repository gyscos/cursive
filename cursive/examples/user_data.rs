//! This example shows the usage of user data.
//!
//! This lets you attach data to the main `Cursive` root, which can simplify
//! communication in simple applications.
//!
//! `Cursive::set_user_data` is used to store or update the user data, while
//! `Cursive::user_data` and `Cursive::with_user_data` can access it, if they
//! know the exact type.
use cursive::views::Dialog;
use cursive::Cursive;

struct Data {
    counter: u32,
}

fn main() {
    let mut siv = cursive::default();

    // `Cursive::set_user_data` accepts any `T: Any`, which includes most
    // owned types
    siv.set_user_data(Data { counter: 0 });

    siv.add_layer(
        Dialog::text("This uses some user data!")
            .title("User data example")
            .button("Increment", |s| {
                // `Cursive::with_user_data()` is an easy way to run a closure
                // on the data.
                s.with_user_data(|data: &mut Data| {
                    data.counter += 1;
                });
            })
            .button("Show", |s| {
                // `Cursive::user_data()` returns a reference to the data.
                let value = s.user_data::<Data>().unwrap().counter;
                s.add_layer(Dialog::info(format!("Current value: {value}")));
            })
            .button("Quit", Cursive::quit),
    );

    siv.run();
}
