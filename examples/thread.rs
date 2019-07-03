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
    // if you precede the thread spawn with a call to
    // markup without thread, this call will work
    // markup();
    // This will never work, for now at least.
    thread::spawn(|| markup());
    // if you follow this statement by a call of
    // markup without thread, it will be *very* confused
    // like a cat trying to find it's image behind the mirror
    // markup();
}
