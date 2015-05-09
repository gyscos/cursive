Cursive
=======

Cursive is a ncurses-based TUI (Text User Interface) library for rust. It is based on jeaye's [ncurses-rs](https://github.com/jeaye/ncurses-rs).

It is designed to be safe, and easy to use:

```rust
extern crate cursive;

use cursive::{Cursive,Dialog};

fn main() {
	let mut siv = Cursive::new();

	// Create a popup window with a "Ok" button that quits the application
	siv.add_layer(Dialog::new("Hello world!").button("Ok", |s| s.quit()));

	// Starts the event loop.
	siv.run();
}
```
