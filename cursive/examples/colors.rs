use cursive::theme::{Color, ColorStyle};
use cursive::view::Resizable;
use cursive::views::Canvas;
use cursive::Printer;

// This example will draw a colored square with a gradient.
//
// We'll use a Canvas, which lets us only define a draw method.
//
// We will combine 2 gradients: one for the background,
// and one for the foreground.
//
// Note: color reproduction is not as good on all backends.
// termion can do full 16M true colors, but ncurses is currently limited to
// 256 colors.

fn main() {
    // Start as usual
    let mut siv = cursive::default();
    siv.add_global_callback('q', |s| s.quit());

    // Canvas lets us easily override any method.
    // Canvas can have states, but we don't need any here, so we use `()`.
    siv.add_layer(Canvas::new(()).with_draw(draw).fixed_size((20, 10)));

    siv.run();
}

/// Method used to draw the cube.
///
/// This takes as input the Canvas state and a printer.
fn draw(_: &(), p: &Printer) {
    // We use the view size to calibrate the color
    let x_max = p.size.x as u8;
    let y_max = p.size.y as u8;

    // Print each cell individually
    for x in 0..x_max {
        for y in 0..y_max {
            // We'll use a different style for each cell
            let style = ColorStyle::new(
                front_color(x, y, x_max, y_max),
                back_color(x, y, x_max, y_max),
            );

            p.with_color(style, |printer| {
                printer.print((x, y), "+");
            });
        }
    }
}

// Gradient for the front color
fn front_color(x: u8, y: u8, x_max: u8, y_max: u8) -> Color {
    // We return a full 24-bits RGB color, but some backends
    // will project it to a 256-colors palette.
    Color::Rgb(
        x * (255 / x_max),
        y * (255 / y_max),
        (x + 2 * y) * (255 / (x_max + 2 * y_max)),
    )
}

// Gradient for the background color
fn back_color(x: u8, y: u8, x_max: u8, y_max: u8) -> Color {
    // Let's try to have a gradient in a different direction than the front color.
    Color::Rgb(
        128 + (2 * y_max + x - 2 * y) * (128 / (x_max + 2 * y_max)),
        255 - y * (255 / y_max),
        255 - x * (255 / x_max),
    )
}
