use cursive::traits::*;

fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        cursive::views::Dialog::new().content(
            cursive::views::LinearLayout::vertical()
                .child(cursive::views::TextView::new("Focused").with_name("text"))
                .child(
                    cursive::views::EditView::new()
                        .wrap_with(cursive::views::FocusTracker::new)
                        .on_focus(|_| {
                            cursive::event::EventResult::with_cb(|s| {
                                s.call_on_name("text", |v: &mut cursive::views::TextView| {
                                    v.set_content("Focused");
                                });
                            })
                        })
                        .on_focus_lost(|_| {
                            cursive::event::EventResult::with_cb(|s| {
                                s.call_on_name("text", |v: &mut cursive::views::TextView| {
                                    v.set_content("Focus lost");
                                });
                            })
                        }),
                )
                .child(cursive::views::Button::new("Quit", |s| s.quit()))
                .fixed_width(20),
        ),
    );

    siv.run();
}
