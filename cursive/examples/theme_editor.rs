use cursive::style::{BaseColor, BorderStyle, Color, Palette, PaletteColor};
use cursive::theme::Theme;
use cursive::traits::{Finder, Nameable, Resizable, With};

#[derive(Clone, Copy)]
enum ColorKind {
    TerminalDefault,
    Base,
    Rgb,
}

fn set_color_subtree(v: &mut cursive::views::LinearLayout, kind: &ColorKind, title: &str) {
    // First, clean out all child views here.
    while v.len() > 1 {
        v.remove_child(1);
    }
    match kind {
        ColorKind::TerminalDefault => {
            // No need for any extra child there.
        }
        ColorKind::Base => {
            v.add_child(cursive::views::DummyView.fixed_width(2));
            v.add_child(
                cursive::views::SelectView::new()
                    .popup()
                    .with_all(BaseColor::all().map(|color| (format!("{color:?}"), color)))
                    .on_submit(|s, _| apply(s))
                    .with_name(format!("{title}_base")),
            );

            v.add_child(cursive::views::DummyView.fixed_width(2));
            v.add_child(cursive::views::TextView::new("Light:"));
            v.add_child(cursive::views::DummyView);

            v.add_child(
                cursive::views::Checkbox::new()
                    .on_change(|s, _| apply(s))
                    .with_name(format!("{title}_light")),
            )
        }
        ColorKind::Rgb => {
            v.add_child(cursive::views::DummyView.fixed_width(2));
            for primary in ["red", "green", "blue"] {
                v.add_child(
                    cursive::views::EditView::new()
                        .max_content_width(3)
                        .on_edit(|s, _, _| apply(s))
                        .with_name(format!("{title}_{primary}"))
                        .fixed_width(4),
                );
                v.add_child(cursive::views::DummyView);
            }
            v.add_child(cursive::views::DummyView);
            v.add_child(cursive::views::TextView::new("Low Res:"));
            v.add_child(cursive::views::DummyView);
            v.add_child(
                cursive::views::Checkbox::new()
                    .on_change(|s, _| apply(s))
                    .with_name(format!("{title}_lowres")),
            )
        }
    }
}

fn make_color_selection(title: &str, starting_color: Color) -> impl cursive::View {
    cursive::views::LinearLayout::horizontal()
        .child(
            cursive::views::SelectView::new()
                .popup()
                .item("Terminal default", ColorKind::TerminalDefault)
                .item("Base color", ColorKind::Base)
                .item("RGB", ColorKind::Rgb)
                .on_submit({
                    let title = title.to_string();
                    move |s, kind| {
                        let title = &title;
                        s.call_on_name(title, move |v: &mut cursive::views::LinearLayout| {
                            set_color_subtree(v, kind, title);
                        })
                        .unwrap();
                        apply(s);
                    }
                })
                .selected(match starting_color {
                    Color::TerminalDefault => 0,
                    Color::Dark(_) | Color::Light(_) => 1,
                    Color::Rgb(..) | Color::RgbLowRes(..) => 2,
                })
                .with_name(format!("{title}_kind")),
        )
        .with(|v| match starting_color {
            Color::TerminalDefault => (),
            Color::Rgb(r, g, b) | Color::RgbLowRes(r, g, b) => {
                let is_lowres = matches!(starting_color, Color::RgbLowRes(..));
                set_color_subtree(v, &ColorKind::Rgb, title);

                for (primary, value) in [("red", r), ("green", g), ("blue", b)] {
                    v.call_on_name(
                        &format!("{title}_{primary}"),
                        |e: &mut cursive::views::EditView| {
                            e.set_content(format!("{value}"));
                        },
                    )
                    .unwrap();
                }

                v.call_on_name(
                    &format!("{title}_lowres"),
                    |c: &mut cursive::views::Checkbox| {
                        c.set_checked(is_lowres);
                    },
                )
                .unwrap();
            }
            Color::Light(color) | Color::Dark(color) => {
                let is_light = matches!(starting_color, Color::Light(_));
                set_color_subtree(v, &ColorKind::Base, title);
                v.call_on_name(
                    &format!("{title}_base"),
                    |s: &mut cursive::views::SelectView<BaseColor>| {
                        s.set_selection(color as usize);
                    },
                )
                .unwrap();

                v.call_on_name(
                    &format!("{title}_light"),
                    |c: &mut cursive::views::Checkbox| {
                        c.set_checked(is_light);
                    },
                )
                .unwrap();
            }
        })
        .with_name(title)
}

fn make_dialog(theme: &Theme) -> impl cursive::View {
    cursive::views::Dialog::new()
        .title("Theme editor")
        .button("Quit", |s| s.quit())
        .content(
            cursive::views::LinearLayout::vertical()
                .child(
                    cursive::views::ListView::new()
                        .child(
                            "Shadows",
                            cursive::views::Checkbox::new()
                                .with_checked(theme.shadow)
                                .on_change(|s, _| apply(s))
                                .with_name("shadows"),
                        )
                        .child(
                            "Borders",
                            cursive::views::SelectView::new()
                                .popup()
                                .with_all(BorderStyle::all().map(|b| (format!("{b:?}"), b)))
                                .selected(theme.borders as usize)
                                .on_submit(|s, _| apply(s))
                                .with_name("borders")
                                .max_width(10),
                        )
                        .child(
                            "Palette",
                            cursive::views::ListView::new().with(|l| {
                                for color in PaletteColor::all() {
                                    let current_color = theme.palette[color];
                                    let color = format!("{color:?}");
                                    l.add_child(
                                        &color,
                                        make_color_selection(&color, current_color),
                                    );
                                }
                            }),
                        ),
                )
                .child(cursive::views::DummyView)
                .child(
                    cursive::views::TextView::new("Press R to reset the theme.")
                        .style(BaseColor::Red.light())
                        .h_align(cursive::align::HAlign::Center)
                        .with_name("status"),
                ),
        )
        .fixed_width(80)
}

fn main() {
    let mut siv = cursive::default();

    siv.add_global_callback('r', |s| {
        let theme = Theme::default();
        s.pop_layer();
        s.add_layer(make_dialog(&theme));
        s.set_theme(theme);
    });

    let theme = siv.current_theme().clone();

    siv.add_layer(make_dialog(&theme));

    siv.run();
}

fn get_edit_text(siv: &mut cursive::Cursive, name: &str) -> Option<u8> {
    siv.call_on_name(name, |e: &mut cursive::views::EditView| {
        e.get_content().parse()
    })
    .unwrap()
    .ok()
}

fn get_checkbox(siv: &mut cursive::Cursive, name: &str) -> bool {
    siv.call_on_name(name, |s: &mut cursive::views::Checkbox| s.is_checked())
        .unwrap()
}

fn get_selection<T: Send + Sync + Copy + 'static>(siv: &mut cursive::Cursive, name: &str) -> T {
    siv.call_on_name(name, |s: &mut cursive::views::SelectView<T>| {
        *s.selection().unwrap()
    })
    .unwrap()
}

fn find_color(siv: &mut cursive::Cursive, title: &str) -> Option<Color> {
    // First, the kind
    Some(match get_selection(siv, &format!("{title}_kind")) {
        ColorKind::TerminalDefault => Color::TerminalDefault,
        ColorKind::Base => {
            let base_color: BaseColor = get_selection(siv, &format!("{title}_base"));
            let light = get_checkbox(siv, &format!("{title}_light"));
            if light {
                base_color.light()
            } else {
                base_color.dark()
            }
        }
        ColorKind::Rgb => {
            let r = get_edit_text(siv, &format!("{title}_red"));
            let g = get_edit_text(siv, &format!("{title}_green"));
            let b = get_edit_text(siv, &format!("{title}_blue"));
            let lowres = get_checkbox(siv, &format!("{title}_lowres"));
            match (r, g, b) {
                (Some(r), Some(g), Some(b)) => {
                    if lowres {
                        if let Some(color) = Color::low_res(r, g, b) {
                            color
                        } else {
                            siv.call_on_name("status", |t: &mut cursive::views::TextView| {
                                t.set_content(format!(
                                    "Low-resolution values can only be <= 5 for {title}."
                                ))
                            })
                            .unwrap();
                            return None;
                        }
                    } else {
                        Color::Rgb(r, g, b)
                    }
                }
                _ => {
                    // invalid!
                    siv.call_on_name("status", |t: &mut cursive::views::TextView| {
                        t.set_content(format!("Invalid R/G/B values for {title}."))
                    });
                    return None;
                }
            }
        }
    })
}

fn apply(siv: &mut cursive::Cursive) {
    siv.call_on_name("status", |t: &mut cursive::views::TextView| {
        t.set_content("Press R to reset the theme.")
    })
    .unwrap();

    let shadow = siv
        .call_on_name("shadows", |c: &mut cursive::views::Checkbox| c.is_checked())
        .unwrap();

    let borders = siv
        .call_on_name(
            "borders",
            |c: &mut cursive::views::SelectView<BorderStyle>| *c.selection().unwrap(),
        )
        .unwrap();

    let mut palette = Palette::default();
    for color in PaletteColor::all() {
        if let Some(c) = find_color(siv, &format!("{color:?}")) {
            palette[color] = c;
        }
    }

    siv.set_theme(Theme {
        shadow,
        borders,
        palette,
    })
}
