use cursive;

fn main() {
    let mut v = cursive::views::SelectView::new()
        .item_str("Foo")
        .item_str("Bar");

    let (label, content) = v.get_item_mut(1).unwrap();
    *label = "foo".into();
    *content = "foooo".into();

    let mut siv = cursive::default();
    siv.add_layer(v);
    siv.run();
}
