use cursive::align::HAlign;
use cursive::traits::Scrollable;
use cursive::view::{Nameable, Resizable};
use cursive::views::{Dialog, EditView, LinearLayout, SelectView, TextView};
use cursive::Cursive;
use lazy_static::lazy_static;

// This example shows a way to implement a (Google-like) autocomplete search box.
// Try entering "tok"!

lazy_static! {
    static ref CITIES: &'static str = include_str!("assets/cities.txt");
}

fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                // the query box is on the top
                .child(
                    EditView::new()
                        // update results every time the query changes
                        .on_edit(on_edit)
                        // submit the focused (first) item of the matches
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
                        .scrollable(),
                )
                .fixed_height(10),
        )
        .button("Quit", Cursive::quit)
        .title("Where are you from?"),
    );

    siv.run();
}

// Update results according to the query
fn on_edit(siv: &mut Cursive, query: &str, _cursor: usize) {
    let matches = search_fn(CITIES.lines(), query);
    // Update the `matches` view with the filtered array of cities
    siv.call_on_name("matches", |v: &mut SelectView| {
        v.clear();
        v.add_all_str(matches);
    });
}

// Filter cities with names containing query string. You can implement your own logic here!
fn search_fn<'a, 'b, T: std::iter::IntoIterator<Item = &'a str>>(
    items: T,
    query: &'b str,
) -> Vec<&'a str> {
    items
        .into_iter()
        .filter(|&item| {
            let item = item.to_lowercase();
            let query = query.to_lowercase();
            item.contains(&query)
        })
        .collect()
}

fn on_submit(siv: &mut Cursive, query: &str) {
    let matches = siv.find_name::<SelectView>("matches").unwrap();
    if matches.is_empty() {
        // not all people live in big cities. If none of the cities in the list matches, use the value of the query.
        show_next_window(siv, query);
    } else {
        // pressing "Enter" without moving the focus into the `matches` view will submit the first match result
        let city = &*matches.selection().unwrap();
        show_next_window(siv, city);
    };
}

fn show_next_window(siv: &mut Cursive, city: &str) {
    siv.pop_layer();
    let text = format!("{} is a great city!", city);
    siv.add_layer(
        Dialog::around(TextView::new(text)).button("Quit", |s| s.quit()),
    );
}
