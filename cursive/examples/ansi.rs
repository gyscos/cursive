fn main() {
    // Start with some text content that includes ANSI codes.
    // Often this could be the output of another command meant for humans.
    let content = include_str!("text_with_ansi_codes.txt").trim();

    // Parse the content as ANSI-decorated text.
    let styled = cursive::utils::markup::ansi::parse(content);

    // Just give this to `TextView`
    let text_view = cursive::views::TextView::new(styled);

    // And make a minimal app around that.
    let mut siv = cursive::default();
    siv.add_layer(text_view);
    siv.run();
}
