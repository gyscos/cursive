extern crate cursive;

#[macro_use]
extern crate log;

fn main() {
    // Initialize the cursive logger.
    cursive::logger::init();

    // Use some logging macros from the `log` crate.
    error!("Something serious probably happened!");
    warn!("Or did it?");
    debug!("Logger initialized.");
    info!("Starting!");

    let mut siv = cursive::Cursive::default();
    siv.add_layer(cursive::views::Dialog::text("Press ~ to open the console.\nPress l to generate logs.\nPress q to quit."));
    siv.add_global_callback('q', cursive::Cursive::quit);
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);

    siv.add_global_callback('l', |_| trace!("Wooo"));

    siv.run();
}
