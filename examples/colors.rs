extern crate cursive;

use cursive::{Cursive, Printer};
use cursive::theme::{Color, ColorStyle};
use cursive::view::Boxable;
use cursive::views::Canvas;

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
    let mut siv = Cursive::new();

    siv.add_layer(Canvas::new(()).with_draw(draw).fixed_size((20, 10)));

    siv.add_global_callback('q', |s| s.quit());

    siv.run();
}

fn draw(_: &(), p: &Printer) {
    // We use the view size to calibrate the color
    let x_max = p.size.x as u8;
    let y_max = p.size.y as u8;

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
    Color::Rgb(
        x * (255 / x_max),
        y * (255 / y_max),
        (x + 2 * y) * (255 / (x_max + 2 * y_max)),
    )
}

// Gradient for the background color
fn back_color(x: u8, y: u8, x_max: u8, y_max: u8) -> Color {
    Color::Rgb(
        128 + (2 * y_max + x - 2 * y) * (128 / (x_max + 2 * y_max)),
        255 - y * (255 / y_max),
        255 - x * (255 / x_max),
    )
}
