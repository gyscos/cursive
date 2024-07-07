// We'll do some automated tests on interface identical to one in select.rs
//
// To run this example call:
// cargo test --example select_test -- --nocapture

fn main() {
    println!("To run this example call:\n$ cargo test --example select_test -- --nocapture");
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
            select.add_item_str("short \0nul\0 1str");
            select.add_item_str("1\x01\x02\x03\x04\x05\x06\x07\x08thru8");
            select.add_item_str("tab\x09and\x0Anewline");
            select.add_item_str("b\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15thru15");
            select.add_item_str("16\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1Fthru1F");
            select.add_item_str("7F\x7Fonly");
            select.add_item_str("80\u{0080}\u{0081}\u{0082}\u{0083}\u{0084}\u{0085}\u{0086}\u{0087}\u{0088}\u{0089}thru89");
            select.add_item_str("8A\u{008A}\u{008B}\u{008C}\u{008D}\u{008E}\u{008F}\u{0090}\u{0091}\u{0092}\u{0093}thru93");
            select.add_item_str("94\u{0094}\u{0095}\u{0096}\u{0097}\u{0098}\u{0099}\u{009A}\u{009B}\u{009C}\u{009D}thru9D");
            select.add_item_str("9E\u{009E}\u{009F}thru9F");
            //XXX: can't add more lines here, it would cause them to go off view thus fail the is-it-on-screen tests, unless the dialog is made bigger below!

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
        siv.add_layer(Dialog::around(TextView::new(text)).button("Quit", |s| s.quit()));
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
    fn control_chars_including_nul_when_on_screen() {
        let mut s = BasicSetup::new();
        s.hit_keystroke(Key::End);
        let screen = s.last_screen().unwrap();
        s.dump_debug();
        let replacement_char = "\u{FFFD} aka \\u{FFFD}";
        use unicode_width::UnicodeWidthStr;
        let width_of_nul = "\0".width();
        if !(0..=1).contains(&width_of_nul) {
            panic!(
                "nul aka \\0 has a width of '{width_of_nul}' instead of the expected one of 0 or 1"
            );
        }
        //we assume that all other control chars have same width as nul for chosing
        //which test to perform on them which depends on which unicode-width crate
        //version was used: the <=0.1.12 (width==0) or >=0.1.13 (width==1)
        //for width==0 we expect control chars to have been deleted from on-screen output
        //for width==1 we expect they were replaced with the width 1 replacement char: �
        if width_of_nul == 1 {
            // unicode-width version =1.1.13 or maybe later too
            assert_eq!(
                screen
                    .find_occurences("short \u{fffd}nul\u{FFFD} 1str")
                    .len(),
                1,
                "nuls aka \\0 in strings are supposed to become the replacement char '\u{fffd}'"
            );
            assert_eq!(
                screen.find_occurences("tab�and�newline").len(),
                1,
                "tabs and newline should've been replaced with replacement char {replacement_char}"
            );
            assert_eq!(
                screen.find_occurences("b�����������thru15").len(),
                1,
                "control chars \\x0B thru \\x15 should've been replaced with the replacement char {replacement_char}",
            );
            assert_eq!(
                screen.find_occurences("16����������thru1F").len(),
                1,
                "control chars \\x16 thru \\x1F should've been replaced with the replacement char {replacement_char}",
            );
            assert_eq!(
                screen.find_occurences("7F�only").len(),
                1,
                "control char \\x7F should've been replaced with the replacement char {replacement_char}",
            );
            assert_eq!(
                screen.find_occurences("80����������thru89").len(),
                1,
                "control chars \\x80 thru \\x89 should've been replaced with the replacement char {replacement_char}",
            );
            assert_eq!(
                screen.find_occurences("8A����������thru93").len(),
                1,
                "control chars \\x8A thru \\x93 should've been replaced with the replacement char {replacement_char}",
            );
            assert_eq!(
                screen.find_occurences("9E��thru9F").len(),
                1,
                "control chars \\x9E thru \\x9F should've been replaced with the replacement char {replacement_char}",
            );
        } else if width_of_nul == 0 {
            // unicode-width version <=1.1.12
            assert_eq!(
                screen.find_occurences("short nul 1str").len(),
                1,
                "nuls aka \\0 in strings are supposed to deleted from output"
            );
            assert_eq!(
                screen.find_occurences("tabandnewline").len(),
                1,
                "tabs and newline should've been deleted from output"
            );
            assert_eq!(
                screen.find_occurences("bthru15").len(),
                1,
                "control chars \\x0B thru \\x15 should've been deleted from output"
            );
            assert_eq!(
                screen.find_occurences("16thru1F").len(),
                1,
                "control chars \\x16 thru \\x1F should've been deleted from output"
            );
            assert_eq!(
                screen.find_occurences("7Fonly").len(),
                1,
                "control char \\x7F should've been deleted from output"
            );
            assert_eq!(
                screen.find_occurences("80thru89").len(),
                1,
                "control chars \\x80 thru \\x89 should've been deleted from output"
            );
            assert_eq!(
                screen.find_occurences("8Athru93").len(),
                1,
                "control chars \\x8A thru \\x93 should've been deleted from output"
            );
            assert_eq!(
                screen.find_occurences("9Ethru9F").len(),
                1,
                "control chars \\x9E thru \\x9F should've been deleted from output"
            );
        } else {
            unreachable!();
        }
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
