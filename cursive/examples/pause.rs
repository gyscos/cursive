use cursive::{self, views};

fn main() {
    let mut siv = cursive::default();

    siv.add_layer(views::Dialog::text("Please write your message.").button("Ok", |s| s.quit()));

    siv.run();
    // At this point the terminal is cleaned up.
    // We can write to stdout like any CLI program.
    // You could also start $EDITOR, or run other commands.

    println!("Enter your message here:");

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();

    // And we can start another event loop later on.
    siv.add_layer(
        views::Dialog::text(format!("Your message was:\n{line}")).button("I guess?", |s| s.quit()),
    );
    siv.run();
}
