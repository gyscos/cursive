extern crate cursive;

use cursive::prelude::*;

fn main() {
    let mut siv = Cursive::new();
    // You can load a theme from a file at runtime for fast development.
    siv.load_theme_file("assets/style.toml").unwrap();

    // Or you can directly load it from a string for easy deployment.
    // siv.load_theme(include_str!("../assets/style.toml")).unwrap();

    siv.add_layer(Dialog::new(TextView::new("This application uses a custom theme!"))
                      .title("Themed dialog")
                      .button("Quit", |s| s.quit()));

    siv.run();
}
