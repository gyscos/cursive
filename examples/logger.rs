extern crate cursive;

#[macro_use]
extern crate log;

fn main() {
    cursive::logger::init();
    debug!("Starting!");

    let mut siv = cursive::Cursive::default();
    siv.add_global_callback('q', cursive::Cursive::quit);
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_view);
    siv.add_global_callback('l', |_| debug!("Wooo"));
    error!("BAD!!!");

    siv.run();
}
