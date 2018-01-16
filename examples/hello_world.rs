extern crate cursive;

use cursive::Cursive;
use cursive::views::TextView;

fn main() {
    let mut siv = Cursive::new();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    // Add a simple view
    siv.add_layer(TextView::new(
        "Hello World!\n\
         Press q to quit the application.",
    ));

    // Run the event loop
    siv.run();
}
