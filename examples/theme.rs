use cursive::views::{Dialog, TextView};

fn main() {
    let mut siv = cursive::default();

    // You can load a theme from a file at runtime for fast development.
    siv.load_theme_file("examples/assets/style.toml").unwrap();

    // Or you can directly load it from a string for easy deployment.
    siv.load_toml(include_str!("assets/style.toml")).unwrap();

    siv.add_layer(
        Dialog::around(TextView::new(
            "This application uses a \
             custom theme!",
        ))
        .title("Themed dialog")
        .button("Oh rly?", |_| ())
        .button("Quit", |s| s.quit()),
    );

    siv.run();
}
