use cursive::align::HAlign;
use cursive::event::EventResult;
use cursive::traits::*;
use cursive::view::{Boxable, Identifiable};
use cursive::views::{
    Dialog, EditView, LinearLayout, OnEventView, SelectView, TextView,
};
use cursive::Cursive;
use lazy_static::lazy_static;

// This example shows a way to implement a (Google-like) autocomplete search box.
// Try entering "Tok"!

lazy_static! {
    static ref CITIES: &'static str = include_str!("../../assets/cities.txt");
}

fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                // the input is on the top
                .child(
                    EditView::new()
                        .on_edit(on_edit)
                        .on_submit(on_submit)
                        .with_name("query"),
                )
                // search results below the input
                .child(
                    SelectView::new()
                        // shows all cities by default
                        .with_all_str(CITIES.lines())
                        // Sets the callback for when "Enter" is pressed.
                        .on_submit(show_next_window)
                        // Center the text horizontally
                        .h_align(HAlign::Center)
                        .with_name("matches")
                        .scrollable()
                        .fixed_size((20, 10)),
                )
                .fixed_width(25),
        )
        .button("Quit", Cursive::quit),
    );

    siv.run();
}

// Update results according to the query
fn on_edit(siv: &mut Cursive, _content: &str, _cursor: usize) {
    // Get the query
    let query = siv.find_name::<EditView>("query").unwrap().get_content();
    // Filter cities with names containing query string
    let matches = CITIES.lines().filter(|&city| {
        let city = city.to_owned().to_lowercase();
        let query = query.to_owned().to_lowercase();
        city.contains(&query)
    });
    // Update the `matches` view with the filtered array of cities
    siv.call_on_name("matches", |v: &mut SelectView| {
        v.clear();
        v.add_all_str(matches);
    });
}

fn on_submit(siv: &mut Cursive, _content: &str) {
    siv.call_on_name("matches", |v: &mut SelectView| {
        // wont' work; private associated function
        // v.submit();
    });
}

fn show_next_window(siv: &mut Cursive, city: &str) {
    siv.pop_layer();
    let text = format!("{} is a great city!", city);
    siv.add_layer(
        Dialog::around(TextView::new(text)).button("Quit", |s| s.quit()),
    );
}
