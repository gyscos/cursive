// Automated rendering test for `ProgressBar`, using the puppet backend.
//
// This checks that, in a default (non-themed) application, the progress bar
// renders the expected block/label layout.

use cursive::backends::puppet::observed::{ObservedPieceInterface, ObservedScreen};
use cursive::event::Event;
use cursive::traits::*;
use cursive::utils::Counter;
use cursive::views::*;
use cursive::*;

fn render_progress_bar(value: usize, width: usize) -> ObservedScreen {
    // Default `Cursive::new()` uses the default (non-themed) palette.
    let bar = ProgressBar::new()
        .with_value(Counter::new(value))
        .fixed_width(width);

    // Give the layer some vertical room: with only 1 row the (invisible)
    // dialog/layer chrome doesn't get a chance to lay out. The bar's
    // width exactly matches the screen width, so there's no horizontal
    // centering offset to account for.
    let size = Vec2::new(width, 3);
    let backend = backends::puppet::Backend::init(Some(size));
    let sink = backend.stream();
    let input = backend.input();
    let mut siv = Cursive::new().into_runner(backend);

    siv.add_layer(bar);

    input.send(Some(Event::Refresh)).unwrap();
    siv.step();

    let mut last_screen = None;
    while let Ok(screen) = sink.try_recv() {
        last_screen = Some(screen);
    }
    last_screen.expect("expected at least one rendered frame")
}

#[test]
fn renders_expected_layout_at_50_percent() {
    // width=10, value=50 (min=0, max=100) => filled length=5, no fraction.
    // label = "50 %" centered => starts at offset 3.
    let screen = render_progress_bar(50, 10);
    screen.print_stdout();

    let rows = screen.as_strings();
    assert_eq!(rows.len(), 3);
    // The left half (filled) shows the start of the label ("50"),
    // the right half (empty) shows the rest (" %"), with a blank/
    // block transition character at the boundary (index 5).
    assert_eq!(rows[1], "   50 %   ");
}

#[test]
fn renders_expected_layout_at_0_percent() {
    let screen = render_progress_bar(0, 10);
    screen.print_stdout();

    let rows = screen.as_strings();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1], "   0 %    ");
}

#[test]
fn renders_expected_layout_at_100_percent() {
    let screen = render_progress_bar(100, 10);
    screen.print_stdout();

    let rows = screen.as_strings();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1], "  100 %   ");
}
