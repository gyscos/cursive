struct StyledEditor {
    // Style of the next entered character
    current_style: cursive::theme::Style,

    // Text we are editing
    content: cursive::utils::markup::StyledString,

    // Cached rows of content
    rows: Vec<cursive::utils::lines::spans::Row>,

    // Help maintain a valid row cache
    size_cache: Option<cursive::XY<cursive::view::SizeCache>>,

    // Scroll the editor view
    core: cursive::view::scroll::Core,
}

fn main() {
    let mut siv = cursive::default();

    siv.run();
}
