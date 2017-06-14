extern crate cursive;

use cursive::Cursive;
use cursive::theme::{ColorStyle, Color};
use cursive::view::Boxable;
use cursive::views::Canvas;

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(Canvas::new(())
                      .with_draw(|printer, _| for x in 0..20 {
                                     for y in 0..10 {
            printer.with_color(ColorStyle::Custom {
                                   front: Color::Rgb(x * 12,
                                                     y * 25,
                                                     (x + 2 * y) * 6),
                                   back: Color::Rgb(255 - x * 12,
                                                    255 - y * 25,
                                                    128 + (40 - x - 2 * y) * 3),
                               },
                               |printer| { printer.print((x, y), "+"); });
        }
                                 })
                      .fixed_size((20, 10)));

    siv.add_global_callback('q', |s| s.quit());

    siv.run();
}
