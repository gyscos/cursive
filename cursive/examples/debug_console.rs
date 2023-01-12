fn main() {
    // Initialize the cursive logger.
    cursive::logger::init();

    // Use some logging macros from the `log` crate.
    log::error!("Something serious probably happened!");
    log::warn!("Or did it?");
    log::debug!("Logger initialized.");
    log::info!("Starting!");

    let mut siv = cursive::default();
    siv.add_layer(cursive::views::Dialog::text(
        "Press ~ to open the console.\nPress l to generate logs.\nPress q to quit.",
    ));
    siv.add_global_callback('q', cursive::Cursive::quit);
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);

    siv.add_global_callback('l', |_| log::trace!("Wooo"));

    siv.run();
}
