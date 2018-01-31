extern crate cursive;

use cursive::Cursive;
use cursive::views::TextView;
use cursive::views::LayerPosition;
use cursive::view::Position;

fn move_a(mut c: &mut Cursive) {
    let mut s = c.screen_mut();
    let l = LayerPosition::FromFront(0);
    let (x,y) = s.offset().pair();
    let x = x - 1;

    let p = Position::absolute((x,y));
    s.reposition_layer(l, p);
}

fn move_w(mut c: &mut Cursive) {
    let mut s = c.screen_mut();
    let l = LayerPosition::FromFront(0);
    let (x,y) = s.offset().pair();
    let y = y - 1;

    let p = Position::absolute((x,y));
    s.reposition_layer(l, p);
}

fn move_s(mut c: &mut Cursive) {
    {
        let mut s = c.screen_mut();
        let l = LayerPosition::FromFront(0);
        let (x,y) = s.offset().pair();
        let y = y + 1;

        let p = Position::absolute((x,y));
        s.reposition_layer(l, p);
    }
    c.clear();
}

fn move_d(mut c: &mut Cursive) {
    let mut s = c.screen_mut();
    let l = LayerPosition::FromFront(0);
    let (x,y) = s.offset().pair();
    let x = x + 1;

    let p = Position::absolute((x,y));
    s.reposition_layer(l, p);
}

fn main() {
    let mut siv = Cursive::new();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);
    siv.add_global_callback('w', |s| move_w(s));
    siv.add_global_callback('a', |s| move_a(s));
    siv.add_global_callback('s', |s| move_s(s));
    siv.add_global_callback('d', |s| move_d(s));

    // Add a simple view
    siv.add_layer(TextView::new(
        "Press w,a,s,d to move the window.\n\
         Press q to quit the application.",
    ));

    // Run the event loop
    siv.run();
}
