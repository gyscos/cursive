// We'll do some automated tests on interface identical to one in select.rs
//
// To run this example call:
// cargo test --example select_test -- --nocapture

fn main() {
    print!(
        "To run this example call:\n$ cargo test --bin select_test -- --nocapture\n"
    );
}

#[cfg(test)]
pub mod tests {
    extern crate cursive;

    use cursive::align::HAlign;
    use cursive::backends::puppet::observed::ObservedScreen;
    use cursive::event::Event;
    use cursive::event::EventResult;
    use cursive::event::Key;
    use cursive::traits::*;
    use cursive::views::*;
    use cursive::*;
    use std::cell::RefCell;

    pub struct BasicSetup {
        siv: CursiveRunner<Cursive>,
        screen_stream: crossbeam_channel::Receiver<ObservedScreen>,
        input: crossbeam_channel::Sender<Option<Event>>,
        last_screen: RefCell<Option<ObservedScreen>>,
    }

    impl BasicSetup {
        pub fn new() -> Self {
            let mut select = SelectView::new()
                // Center the text horizontally
                .h_align(HAlign::Center)
                // Use keyboard to jump to the pressed letters
                .autojump();

            // Read the list of cities from separate file, and fill the view with it.
            // (We include the file at compile-time to avoid runtime read errors.)
            let content = include_str!("assets/cities.txt");
            select.add_all_str(content.lines());

            // Sets the callback for when "Enter" is pressed.
            select.set_on_submit(show_next_window);

            // Let's override the `j` and `k` keys for navigation
            let select = OnEventView::new(select)
                .on_pre_event_inner('k', |s, _| {
                    s.select_up(1);
                    Some(EventResult::Consumed(None))
                })
                .on_pre_event_inner('j', |s, _| {
                    s.select_down(1);
                    Some(EventResult::Consumed(None))
                });

            let size = Vec2::new(80, 16);
            let backend = backends::puppet::Backend::init(Some(size));
            let sink = backend.stream();
            let input = backend.input();
            let mut siv = Cursive::new().into_runner(backend);

            // Let's add a ResizedView to keep the list at a reasonable size
            // (it can scroll anyway).
            siv.add_layer(
                Dialog::around(select.scrollable().fixed_size((20, 10)))
                    .title("Where are you from?"),
            );

            input.send(Some(Event::Refresh)).unwrap();
            siv.step();

            BasicSetup {
                siv,
                screen_stream: sink,
                input,
                last_screen: RefCell::new(None),
            }
        }

        pub fn last_screen(&self) -> Option<ObservedScreen> {
            while let Ok(screen) = self.screen_stream.try_recv() {
                self.last_screen.replace(Some(screen));
            }

            self.last_screen.borrow().clone()
        }

        pub fn dump_debug(&self) {
            self.last_screen().as_ref().map(|s| s.print_stdout());
        }

        pub fn hit_keystroke(&mut self, key: Key) {
            self.input.send(Some(Event::Key(key))).unwrap();
            self.siv.step();
        }
    }

    // Let's put the callback in a separate function to keep it clean,
    // but it's not required.
    fn show_next_window(siv: &mut Cursive, city: &str) {
        siv.pop_layer();
        let text = format!("{} is a great city!", city);
        siv.add_layer(
            Dialog::around(TextView::new(text)).button("Quit", |s| s.quit()),
        );
    }

    #[test]
    fn displays() {
        let s = BasicSetup::new();
        let screen = s.last_screen().unwrap();
        s.dump_debug();
        assert_eq!(screen.find_occurences("Where are you from").len(), 1);
        assert_eq!(screen.find_occurences("Some random string").len(), 0);
    }

    #[test]
    fn interacts() {
        let mut s = BasicSetup::new();
        s.hit_keystroke(Key::Down);
        s.hit_keystroke(Key::Enter);

        let screen = s.last_screen().unwrap();
        s.dump_debug();
        assert_eq!(
            screen.find_occurences("Abu Dhabi is a great city!").len(),
            1
        );
        assert_eq!(screen.find_occurences("Abidjan").len(), 0);
    }
}
