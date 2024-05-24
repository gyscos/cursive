use cursive::views;

fn main() {
    let mut siv = cursive::default();

    siv.clear_global_callbacks(cursive::event::Event::CtrlChar('c'));

    siv.set_on_pre_event(cursive::event::Event::CtrlChar('c'), |s| {
        s.add_layer(
            views::Dialog::text("Do you want to quit?")
                .button("Yes", |s| s.quit())
                .button("No", |s| {
                    s.pop_layer();
                }),
        );
    });

    siv.add_layer(views::Dialog::text("Try pressing Ctrl-C!"));

    siv.run();
}
