use cursive::event::{Event, EventResult, Key, MouseEvent};
use cursive::theme::{BaseColor, Color, ColorPair, ColorStyle};
use cursive::traits::*;
use cursive::utils::markup::cursup;
use cursive::views::{Canvas, Panel};
use cursive::Rect;

// left: tools
// - Rect
//  - color (back color) (4-wide)
//      - Terminal default
//      - 0-7
//      - 8-15
//      - RGB
//  - edges
//      - None
//      - Simple
//      - Outset
//      - + front color
// - Text
// - marker
// - colors (palette?)
// center: image
// right: layers
//
struct App {
    tool: Tool,
    // color: ColorPair,
    // palette: Vec<ColorPair>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Tool {
    Rectangle,
    Text,
    Marker,
}

enum Edge {
    None,
    Simple,
    Outset,
    Repeat(String),
}

enum ToolConfig {
    Rectangle { edge: Edge },
    Text,
    Marker { text: String },
}

fn tools_select() -> impl cursive::View {
    cursive::views::SelectView::new()
        .item(cursup::parse("/bold+red{R}ectangle"), Tool::Rectangle)
        .item(cursup::parse("/bold+green{T}ext"), Tool::Text)
        .item(cursup::parse("/bold+blue{M}arker"), Tool::Marker)
        .on_select(|s, t| {
            s.with_user_data(|data: &mut App| {
                // foo
                data.tool = *t;
            })
            .unwrap();
        })
}

fn main() {
    let mut siv = cursive::default();
    siv.set_user_data(App {
        tool: Tool::Rectangle,
    });

    siv.screen_mut().add_transparent_layer(
        cursive::views::LinearLayout::horizontal()
            .child(
                cursive::views::ShadowView::new(cursive::views::Layer::new(
                    cursive::views::Panel::new(tools_select()).title("Tools"),
                ))
                .fixed_size((32, 22)),
            )
            .child(cursive::views::DummyView.fixed_size((1, 1)))
            .child(cursive::views::ShadowView::new(cursive::views::Layer::new(
                cursive::views::Panel::new(cursive::views::DummyView.fixed_size((80, 24)))
                    .title("Image"),
            )))
            .child(cursive::views::DummyView.fixed_size((1, 1)))
            .child(
                cursive::views::ShadowView::new(cursive::views::Layer::new(
                    cursive::views::Panel::new(cursive::views::DummyView).title("Layers"),
                ))
                .fixed_size((22, 22)),
            ),
    );

    fn zones() -> impl Iterator<Item = (usize, Rect)> {
        0..16.map(|i| {
            let x = i % 8;
            let y = i / 8;

            (i, Rect::from_size((x, y), (2, 1)))
        })
    }

    siv.add_layer(
        Panel::new(
            Canvas::new(0)
                .with_draw(|selection, printer| {
                    printer.with_color(ColorStyle::back(Color::from_256colors(15)), |printer| {
                        printer.print_rect(Rect::from_size((1, 0), (3 * 8 + 1, 3)), " ");
                    });
                    printer.with_color(ColorStyle::back(Color::from_256colors(0)), |printer| {
                        printer.print_rect(Rect::from_size((1, 3), (3 * 8 + 1, 3)), " ");
                    });
                    for (i, zone) in zones() {
                        let alternate = if i < 8 { 15 } else { 0 };
                        printer.with_color(
                            ColorStyle {
                                front: Color::from_256colors(alternate).into(),
                                back: Color::from_256colors(i).into(),
                            },
                            |printer| {
                                printer.print_rect(rect, if i == *selection { "🬇🬃" } else { "  " });
                            },
                        );
                    }
                })
                .with_on_event(|selection, event| {
                    match event {
                        Event::Key(Key::Up) if *selection >= 8 => {
                            *selection -= 8;
                        }
                        Event::Key(Key::Down) if *selection < 8 => {
                            *selection += 8;
                        }
                        Event::Key(Key::Left) if (*selection % 8) >= 1 => {
                            *selection -= 1;
                        }
                        Event::Key(Key::Right) if (*selection % 8) < 7 => {
                            *selection += 1;
                        }
                        Event::Mouse {
                            event: MouseEvent::Press(_),
                            position,
                            offset,
                        } => {
                            let Some(position) = position.checked_sub(offset) else {
                                return EventResult::Ignored;
                            };

                            let Some(y) = position.y.checked_sub(1) else {
                                return EventResult::Ignored;
                            };
                            if y % 2 == 1 {
                                return EventResult::Ignored;
                            }

                            let Some(x) = position.x.checked_sub(2) else {
                                return EventResult::Ignored;
                            };
                            if x % 3 == 2 {
                                return EventResult::Ignored;
                            }

                            let selected = x / 3 + (y / 2) * 8;
                        }
                        _ => return EventResult::Ignored,
                    }
                    EventResult::consumed()
                })
                .fixed_size((3 * 8 + 3, 6)),
        )
        .title("Palette"),
    );

    siv.run();
}
