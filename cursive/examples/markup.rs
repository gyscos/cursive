use cursive::style::BaseColor;
use cursive::style::Color;
use cursive::style::Effect;
use cursive::style::Style;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, TextView};

fn main() {
    let mut siv = cursive::default();

    let mut styled = StyledString::plain("Isn't ");
    styled.append(StyledString::styled("that ", Color::Dark(BaseColor::Red)));
    styled.append(StyledString::styled(
        "cool?",
        Style::from(Color::Light(BaseColor::Blue)).combine(Effect::Bold),
    ));

    // TextView can natively accept StyledString.
    siv.add_layer(Dialog::around(TextView::new(styled)).button("Hell yeah!", |s| s.quit()));

    siv.run();
}
