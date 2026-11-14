//! Checks that layout keys notice every change that affects a view's size.
//!
//! A long-lived tree is mutated through each view's public API, and must
//! always give the same sizes as a tree built from scratch: a key that
//! misses a change lets a container reuse a stale size.

use crate::{
    Vec2,
    direction::Orientation,
    view::{Finder, Nameable, View},
    views::{
        Button, Dialog, EditView, LinearLayout, ListView, ScrollView, SelectView, SliderView,
        StackView, TextArea, TextView,
    },
};

/// Checks that a view and a layout holding it can cache their layout.
///
/// With nothing changing, the key must be the same from one call to the next
/// (a new key every time means no container can ever reuse a size),
/// and no view may ask for a relayout once laid out.
pub(crate) fn assert_layout_is_cached<V: View>(make: impl Fn() -> V) {
    let mut view = make();
    assert_eq!(view.layout_key(), view.layout_key(), "key of the view");
    let size = view.required_size(Vec2::new(30, 10));
    view.layout(size);
    assert!(!view.needs_relayout(), "needs_relayout after layout");
    assert_eq!(view.layout_key(), view.layout_key(), "key after layout");

    let mut layout = LinearLayout::vertical().child(make()).child(make());
    assert_eq!(
        layout.layout_key(),
        layout.layout_key(),
        "key of the layout"
    );
    let size = layout.required_size(Vec2::new(30, 10));
    layout.layout(size);
    assert!(
        !layout.needs_relayout(),
        "layout needs_relayout after layout"
    );
    assert_eq!(
        layout.layout_key(),
        layout.layout_key(),
        "layout key after layout"
    );
}

#[derive(Clone)]
struct State {
    edit: String,
    select: Vec<String>,
    popup: bool,
    area: String,
    button: String,
    list: Vec<(String, String)>,
    title: String,
    buttons: Vec<String>,
    scroll_x: bool,
    layers: Vec<String>,
    slider: usize,
}

// In a horizontal layout widths add up, in a vertical one heights do: with
// both, every change in any view's size shows in the root's size.
fn build(state: &State, orientation: Orientation) -> LinearLayout {
    let mut select = SelectView::<String>::new();
    select.add_all_str(state.select.iter().cloned());
    select.set_popup(state.popup);

    let mut list = ListView::new();
    for (label, text) in &state.list {
        list.add_child(label.as_str(), TextView::new(text.as_str()));
    }

    let mut dialog = Dialog::around(TextView::new("dialog body text")).title(state.title.as_str());
    for label in &state.buttons {
        dialog.add_button(label.as_str(), |_| ());
    }

    let mut stack = StackView::new();
    for text in &state.layers {
        stack.add_layer(TextView::new(text.as_str()));
    }

    let mut scroll = ScrollView::new(TextView::new(
        "a long line of text that would rather not wrap\nand more",
    ));
    scroll.set_scroll_x(state.scroll_x);

    LinearLayout::new(orientation)
        .child(
            EditView::new()
                .content(state.edit.as_str())
                .with_name("edit"),
        )
        .child(select.with_name("select"))
        .child(
            TextArea::new()
                .content(state.area.as_str())
                .with_name("area"),
        )
        .child(Button::new(state.button.as_str(), |_| ()).with_name("button"))
        .child(list.with_name("list"))
        .child(dialog.with_name("dialog"))
        .child(scroll.with_name("scroll"))
        .child(stack.with_name("stack"))
        .child(SliderView::new(Orientation::Horizontal, state.slider).with_name("slider"))
}

fn check_all_views(mut seed: u64, frames: usize) {
    let mut rnd = move |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    let words = [
        "a",
        "few",
        "words",
        "that wrap",
        "中文",
        "longer text here",
        "",
    ];
    let text = |rnd: &mut dyn FnMut(usize) -> usize| {
        (0..rnd(5))
            .map(|_| words[rnd(words.len())])
            .collect::<Vec<_>>()
            .join(" ")
    };

    let mut state = State {
        edit: "hello".into(),
        select: vec!["one".into(), "two".into()],
        popup: false,
        area: "some text".into(),
        button: "Ok".into(),
        list: vec![("name".into(), "value".into())],
        title: "Title".into(),
        buttons: vec!["Ok".into()],
        scroll_x: false,
        layers: vec!["layer".into()],
        slider: 5,
    };
    let orientations = [Orientation::Horizontal, Orientation::Vertical];
    let mut trees = orientations.map(|o| build(&state, o));

    for frame in 0..frames {
        for _ in 0..rnd(3) {
            let t = text(&mut rnd);
            match rnd(12) {
                0 => {
                    state.edit = t.clone();
                    for tree in &mut trees {
                        tree.call_on_name("edit", |v: &mut EditView| v.set_content(t.clone()));
                    }
                }
                1 => {
                    state.select = (0..rnd(4)).map(|_| text(&mut rnd)).collect();
                    let items = state.select.clone();
                    for tree in &mut trees {
                        tree.call_on_name("select", |v: &mut SelectView<String>| {
                            v.clear();
                            v.add_all_str(items.clone());
                        });
                    }
                }
                2 => {
                    state.popup = !state.popup;
                    let popup = state.popup;
                    for tree in &mut trees {
                        tree.call_on_name("select", |v: &mut SelectView<String>| {
                            v.set_popup(popup)
                        });
                    }
                }
                3 => {
                    state.area = t.clone();
                    for tree in &mut trees {
                        tree.call_on_name("area", |v: &mut TextArea| v.set_content(t.clone()));
                    }
                }
                4 => {
                    state.button = t.clone();
                    for tree in &mut trees {
                        tree.call_on_name("button", |v: &mut Button| v.set_label(t.clone()));
                    }
                }
                5 => {
                    let label = text(&mut rnd);
                    state.list.push((label.clone(), t.clone()));
                    for tree in &mut trees {
                        tree.call_on_name("list", |v: &mut ListView| {
                            v.add_child(label.as_str(), TextView::new(t.clone()))
                        });
                    }
                }
                6 if !state.list.is_empty() => {
                    let i = rnd(state.list.len());
                    state.list.remove(i);
                    for tree in &mut trees {
                        tree.call_on_name("list", |v: &mut ListView| {
                            v.remove_child(i);
                        });
                    }
                }
                7 => {
                    state.title = t.clone();
                    for tree in &mut trees {
                        tree.call_on_name("dialog", |v: &mut Dialog| v.set_title(t.clone()));
                    }
                }
                8 => {
                    state.buttons = (0..rnd(3)).map(|_| text(&mut rnd)).collect();
                    let buttons = state.buttons.clone();
                    for tree in &mut trees {
                        tree.call_on_name("dialog", |v: &mut Dialog| {
                            v.clear_buttons();
                            for label in buttons.clone() {
                                v.add_button(label, |_| ());
                            }
                        });
                    }
                }
                9 => {
                    state.scroll_x = !state.scroll_x;
                    let x = state.scroll_x;
                    for tree in &mut trees {
                        tree.call_on_name("scroll", |v: &mut ScrollView<TextView>| {
                            v.set_scroll_x(x)
                        });
                    }
                }
                10 => {
                    if state.layers.len() < 3 && rnd(2) == 0 {
                        state.layers.push(t.clone());
                        for tree in &mut trees {
                            tree.call_on_name("stack", |v: &mut StackView| {
                                v.add_layer(TextView::new(t.clone()))
                            });
                        }
                    } else if !state.layers.is_empty() {
                        state.layers.pop();
                        for tree in &mut trees {
                            tree.call_on_name("stack", |v: &mut StackView| {
                                v.pop_layer();
                            });
                        }
                    }
                }
                11 => {
                    state.slider = 1 + rnd(20);
                    let max = state.slider;
                    for tree in &mut trees {
                        tree.call_on_name("slider", |v: &mut SliderView| v.set_max_value(max));
                    }
                }
                _ => {}
            }
        }

        // Tiny sizes too: some changes only show when views are squeezed.
        let size = if rnd(3) == 0 {
            Vec2::new(rnd(12), rnd(6))
        } else {
            Vec2::new(10 + rnd(400), 5 + rnd(150))
        };
        let layout = rnd(2) == 0;
        for (tree, o) in trees.iter_mut().zip(orientations) {
            if layout {
                tree.layout(size);
            }
            let fresh = build(&state, o).required_size(size);
            assert_eq!(
                tree.required_size(size),
                fresh,
                "{o:?}, frame {frame}, size {size:?}"
            );
        }
    }
}

#[test]
fn all_views_long_lived_match_fresh() {
    for seed in [0x9e37_79b9_7f4a_7c15, 1234, 98765] {
        check_all_views(seed, 600);
    }
}

#[test]
fn scroll_settings_change_the_key() {
    // Scroll settings only change the size in tight spaces, which the
    // randomized test rarely reaches.
    let text = "a long line of text that would rather not wrap\nand more";
    let size = Vec2::new(1, 1);
    let mut tree =
        LinearLayout::vertical().child(ScrollView::new(TextView::new(text)).with_name("s"));
    let before = tree.required_size(size);
    tree.call_on_name("s", |s: &mut ScrollView<TextView>| s.set_scroll_x(true));

    let mut fresh = ScrollView::new(TextView::new(text));
    fresh.set_scroll_x(true);
    let expected = LinearLayout::vertical().child(fresh).required_size(size);
    assert_ne!(before, expected);
    assert_eq!(tree.required_size(size), expected);
}

/// Trees of scroll views, panels, text and layouts.
mod scroll_trees {
    use crate::{
        Vec2,
        direction::Orientation,
        view::View,
        views::{BoxedView, LinearLayout, Panel, ScrollView, TextView},
    };
    #[derive(Clone, Debug)]
    enum T {
        Text(&'static str),
        Scroll(bool, Box<T>),
        Panel(Box<T>),
        L(Orientation, Vec<T>),
    }
    fn build(t: &T) -> BoxedView {
        match t {
            T::Text(s) => BoxedView::boxed(TextView::new(*s)),
            T::Scroll(x, t) => {
                let mut s = ScrollView::new(build(t));
                s.set_scroll_x(*x);
                BoxedView::boxed(s)
            }
            T::Panel(t) => BoxedView::boxed(Panel::new(build(t))),
            T::L(o, cs) => {
                let mut l = LinearLayout::new(*o);
                for c in cs {
                    l.add_child(build(c));
                }
                BoxedView::boxed(l)
            }
        }
    }
    fn make(rnd: &mut dyn FnMut(usize) -> usize, depth: usize) -> T {
        let texts = [
            "a b c",
            "hello world",
            "x",
            "a\nb\nc\nd",
            "longer line here",
            "a\nb",
            "中文 字",
        ];
        match if depth == 0 { 0 } else { rnd(5) } {
            0 => T::Text(texts[rnd(texts.len())]),
            1 => T::Scroll(rnd(2) == 0, Box::new(make(rnd, depth - 1))),
            2 => T::Panel(Box::new(make(rnd, depth - 1))),
            _ => T::L(
                if rnd(2) == 0 {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                },
                (0..1 + rnd(3)).map(|_| make(rnd, depth - 1)).collect(),
            ),
        }
    }
    #[test]
    fn long_lived_scroll_trees_match_fresh() {
        // Scroll views react to the space they get (scrollbars), and cache
        // sizes of their own: nest them in layouts and panels, and compare
        // a long-lived tree with fresh ones.
        let mut seed = 4321u64;
        let mut rnd = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for _ in 0..400 {
            let t = make(&mut rnd, 3);
            let mut tree = build(&t);
            for _ in 0..20 {
                let r = Vec2::new(rnd(30), rnd(14));
                if rnd(2) == 0 {
                    tree.layout(r);
                }
                let fresh = build(&t).required_size(r);
                assert_eq!(tree.required_size(r), fresh, "{t:?} at {r:?}");
            }
        }
    }
}

#[test]
fn progress_bar_and_theme_overlay_cache_their_layout() {
    use crate::views::{Gauge, ProgressBar, ThemeOverlay};

    assert_layout_is_cached(ProgressBar::new);
    assert_layout_is_cached(|| ThemeOverlay::new(Gauge::value(0.5), |_| ()));
    assert_layout_is_cached(|| ThemeOverlay::new(TextView::new("text"), |_| ()));

    // The overlay shares the key of the view it wraps: it follows its size.
    let mut overlay = ThemeOverlay::new(TextView::new("a"), |_| ());
    let key = overlay.layout_key();
    assert_eq!(overlay.layout_key(), key);
    overlay.get_inner_mut().set_content("abc");
    assert_ne!(overlay.layout_key(), key);
    assert_eq!(overlay.required_size(Vec2::new(10, 1)), Vec2::new(3, 1));
}
