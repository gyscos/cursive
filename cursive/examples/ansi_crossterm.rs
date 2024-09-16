use crossterm::style::Stylize;
use cursive_core::views::{Dialog, LinearLayout, TextView};

fn main() {
    // Using the crossterm function for text styling and casting it to a string.
    let crossterm_colored = "Crossterm colored text.".red().to_string();

    // Using this same function with another text, but parsing it as ANSI-decorated content.
    let cursive_colored = cursive::utils::markup::ansi::parse(
        "Cursive colored text (using Crossterm + ANSI)".red().to_string(),
    );

    // Minimal application for text output
    let mut siv = cursive::default();
    siv.add_layer(
        Dialog::new()
            .content(
                LinearLayout::vertical()
                    .child(TextView::new(crossterm_colored))
                    .child(TextView::new(cursive_colored)),
            )
            .button("Quit!", |s| s.quit()),
    );
    siv.run();
}