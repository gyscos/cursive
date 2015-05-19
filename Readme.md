Cursive
=======

Cursive is a ncurses-based TUI (Text User Interface) library for rust. It is based on jeaye's [ncurses-rs](https://github.com/jeaye/ncurses-rs).

It is designed to be safe and easy to use:

```rust
extern crate cursive;

use cursive::{Cursive,Dialog,TextView};

fn main() {
    let mut siv = Cursive::new();

    // Create a popup window with a "Ok" button that quits the application
    siv.add_layer(Dialog::new(TextView::new("Hello world!"))
                    .button("Quit", |s, _| s.quit()));

    // Starts the event loop.
    siv.run();
}
```

A few notes :

* The main focus point is _not_ performance. This is a simple layout library, guys, not [compiz](https://www.google.com/search?q=compiz&tbm=isch) piped into [libcaca](https://www.google.com/search?q=libcaca&tbm=isch). Unless you are running it on your microwave's microcontroller, it's not going to be slow.
* The library is single-threaded. Thus, callback methods are blocking - careful what you're doing in there! Feel free to use threads on your side, though.
