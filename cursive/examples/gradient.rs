use cursive::event::EventResult;
use cursive::style::{
    gradient::{Angled, Bilinear, Linear, Radial},
    Rgb,
};
use cursive::traits::*;
use cursive::utils::markup::gradient;
use cursive::views::{Dialog, GradientView, OnEventView, TextView};
use cursive::XY;

fn main() {
    let mut siv = cursive::default();

    let text = "So many colors! So little time!";
    let text = gradient::decorate_front(text, Rgb::new(255, 0, 0), Rgb::new(0, 0, 255));
    let text = gradient::decorate_back(text, Rgb::new(0, 0, 255), Rgb::new(0, 255, 0));

    // Add a simple view
    siv.add_layer(Dialog::new().content(TextView::new(text)).button(
        gradient::decorate_back("Moar", Rgb::new(255, 0, 0), Rgb::new(255, 255, 0)),
        show_more,
    ));

    // Run the event loop
    siv.run();
}

fn show_more(c: &mut cursive::Cursive) {
    let dialog = Dialog::new()
        .button("Moar", show_more_2)
        .fixed_size((40, 20));

    let interpolator = Radial {
        center: XY::new(-0.1, -0.1),
        gradient: Linear::new(Rgb::from(0xFFFFFF), Rgb::from(0x000000)),
    };

    c.pop_layer();
    c.add_layer(GradientView::new(dialog, interpolator));
}

fn show_more_2(c: &mut cursive::Cursive) {
    let dialog = Dialog::new()
        .button("Moar", show_more_3)
        .fixed_size((40, 20));

    let interpolator = Angled {
        angle_rad: 4f32,
        gradient: Linear::new(Rgb::from(0xFFFFFF), Rgb::from(0x000000)),
    };
    c.pop_layer();
    c.add_layer(
        OnEventView::new(GradientView::new(dialog, interpolator))
            .on_event_inner('q', |g, _| {
                g.interpolator_mut().angle_rad += 0.1;
                Some(EventResult::Consumed(None))
            })
            .on_event_inner('e', |g, _| {
                g.interpolator_mut().angle_rad -= 0.1;
                Some(EventResult::Consumed(None))
            }),
    );
}

fn show_more_3(c: &mut cursive::Cursive) {
    let dialog = Dialog::new()
        .button("Quit", |s| s.quit())
        .fixed_size((40, 20));

    let interpolator = Bilinear {
        top_left: Rgb::new(255, 0, 0).as_f32(),
        top_right: Rgb::new(255, 255, 0).as_f32(),
        bottom_left: Rgb::new(127, 255, 255).as_f32(),
        bottom_right: Rgb::new(0, 0, 0).as_f32(),
    };
    c.pop_layer();
    c.add_layer(GradientView::new(dialog, interpolator));
}
