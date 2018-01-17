extern crate cursive;

use cursive::Cursive;
use cursive::views::{Dialog, TextView};

use cursive::theme::Color;
use cursive::theme::BaseColor;
use cursive::theme::Style;
use cursive::theme::Effect;
use cursive::utils::markup::StyledString;

fn main() {
    let mut siv = Cursive::new();

    let mut styled = StyledString::plain("Isn't ");
    styled.append(StyledString::styled("that ", Color::Dark(BaseColor::Red)));
    styled.append(StyledString::styled(
        "cool?",
        Style::from(Color::Light(BaseColor::Blue)).add(Effect::Bold),
    ));

    // TextView can natively accept StyledString.
    siv.add_layer(
        Dialog::around(TextView::new(styled))
            .button("Hell yeah!", |s| s.quit()),
    );

    siv.run();
}
