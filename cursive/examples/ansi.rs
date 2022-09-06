fn main() {
    let content = include_str!("text_with_ansi_codes.txt");
    let styled = cursive::utils::markup::ansi::parse(content);
    let text_view = cursive::views::TextView::new(styled);
    let mut siv = cursive::default();
    siv.add_layer(text_view);
    siv.run();
}
