extern crate cursive;

use cursive::Cursive;
use cursive::view::{IdView,TextView,Dialog,Selector};

fn main() {
    let mut siv = Cursive::new();

    siv.add_global_callback('q' as i32, |s,_| s.quit());

    siv.add_layer(IdView::new("text", TextView::new("Aaahh\nAaaah\nAaaah\nAaaaah\nAaaaah\nAaaaah\nAaaaah\nAaaaaah\nAaaaah")));

    siv.add_layer(Dialog::new(TextView::new("Tak!"))
                  .button("Change", |s,_| s.find::<TextView>(&Selector::Id("text")).unwrap()
                          .set_content("Bleeeeh\nBleeeeeeeeeeh\nBleeeh") )
                  .dismiss_button("Ok"));

    siv.run();
}
