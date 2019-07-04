use cursive::theme::BaseColor;
use cursive::theme::Color;
use cursive::theme::Effect;
use cursive::theme::Style;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, TextView};
use cursive::Cursive;
use std::thread;

// Don't do this: it is *very* uncool (but still cool to try out)

fn markup() {
    let mut siv = Cursive::default();

    let mut styled = StyledString::plain("Isn't ");
    styled.append(StyledString::styled("that ", Color::Dark(BaseColor::Red)));
    styled.append(StyledString::styled(
        "cool?",
        Style::from(Color::Light(BaseColor::Blue)).combine(Effect::Bold),
    ));

    // TextView can natively accept StyledString.
    siv.add_layer(
        Dialog::around(TextView::new(styled))
            .button("Hell yeah!", |s| s.quit()),
    );

    siv.run();
}

pub fn main() {
    // Better not to use this. For ncurses background
    // it seems to work from Linux (tested in Fedora).
    // But, the other backends might barf.
    let handle = thread::spawn(|| markup());
    // The join is necessary, or the tui might not even
    // finish its initialization before main calls it a day!
    handle.join().unwrap();
}
