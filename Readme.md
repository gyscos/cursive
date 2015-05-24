Cursive
=======

Cursive is a ncurses-based TUI (Text User Interface) library for rust. It is based on jeaye's [ncurses-rs](https://github.com/jeaye/ncurses-rs).

It is designed to be safe and easy to use:

```
[dependencies.cursive]
git = "https://github.com/Gyscos/cursive"
```

```rust
extern crate cursive;

use cursive::Cursive;
use cursive::view::{Dialog,TextView};

fn main() {
    let mut siv = Cursive::new();

    // Create a popup window with a "Ok" button that quits the application
    siv.add_layer(Dialog::new(TextView::new("Hello world!"))
                    .button("Quit", |s| s.quit()));

    // Starts the event loop.
    siv.run();
}
```

The goal is to be flexible enough, so that recreating these kind of tools would be - relatively - easy (at least on the layout front):

* [menuconfig](http://en.wikipedia.org/wiki/Menuconfig#/media/File:Linux_x86_3.10.0-rc2_Kernel_Configuration.png)
* [nmtui](https://access.redhat.com/documentation/en-US/Red_Hat_Enterprise_Linux/7/html/Networking_Guide/sec-Configure_a_Network_Team_Using_the_Text_User_Interface_nmtui.html)

A few notes :

* The main focus point is _not_ performance. This is a simple layout library, guys, not [compiz](https://www.google.com/search?q=compiz&tbm=isch) piped into [libcaca](https://www.google.com/search?q=libcaca&tbm=isch). Unless you are running it on your microwave's microcontroller, it's not going to be slow.
* The library is single-threaded. Thus, callback methods are blocking - careful what you're doing in there! Feel free to use threads on your side, though.
* This goal is _not_ to have an equivalent to every ncurses function. You _can_ access the underlying ncurses window when creating your own custom views, so you can do what you want with that, but the main library will probably only use a subset of the ncurses features.
