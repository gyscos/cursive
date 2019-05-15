# Cursive

[![crates.io](https://meritbadge.herokuapp.com/cursive)](https://crates.io/crates/cursive)
[![Build Status (travis)](https://travis-ci.org/gyscos/Cursive.svg?branch=master)](https://travis-ci.org/gyscos/Cursive)
[![Build status (appveyor)](https://ci.appveyor.com/api/projects/status/uk5pww718jsp5x2l/branch/master?svg=true)](https://ci.appveyor.com/project/gyscos/cursive/branch/master)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Gitter chat](https://badges.gitter.im/gyscos/cursive.png)](https://gitter.im/cursive-rs/cursive)


Cursive is a TUI (Text User Interface) library for rust. It uses ncurses by default, but [other backends are available](https://github.com/gyscos/Cursive/wiki/Backends).

It allows you to build rich user interfaces for terminal applications.

# [Documentation](http://docs.rs/cursive)

It is designed to be safe and easy to use:

```toml
[dependencies]
cursive = "0.12"
```

Or to use the latest git version:

```toml
[dependencies]
cursive = { git = "https://github.com/gyscos/Cursive" }
```

([You will also need ncurses installed.](https://github.com/gyscos/Cursive/wiki/Install-ncurses))

```rust,no_run
extern crate cursive;

use cursive::Cursive;
use cursive::views::{Dialog, TextView};

fn main() {
    // Creates the cursive root - required for every application.
    let mut siv = Cursive::default();

    // Creates a dialog with a single "Quit" button
    siv.add_layer(Dialog::around(TextView::new("Hello Dialog!"))
                         .title("Cursive")
                         .button("Quit", |s| s.quit()));

    // Starts the event loop.
    siv.run();
}
```

[![Cursive dialog example](https://raw.githubusercontent.com/gyscos/Cursive/master/doc/cursive_example.png)](examples/dialog.rs)

Check out the other [examples](https://github.com/gyscos/Cursive/tree/master/examples) to get these results, and more:

<div>
<a href="examples/edit.rs"><img src="https://imgur.com/CQgSwly.png" alt="edit.rs example", width="48%" /></a>
<a href="examples/lorem.rs"><img src="https://imgur.com/hW9M9MV.png" alt="lorem.rs example", width="48%" /></a>
<a href="examples/menubar.rs"><img src="https://imgur.com/xx3lZPz.png" alt="menubar.rs example", width="48%" /></a>
<a href="examples/select.rs"><img src="https://imgur.com/couty0n.png" alt="select.rs example", width="48%" /></a>
<a href="examples/mines/"><img src="https://imgur.com/vNteYyy.png" alt="mines example", width="48%" /></a>
<a href="examples/theme.rs"><img src="https://i.imgur.com/3Yleozc.png" alt="theme.rs example", width="48%" /></a>
</div>

_(Colors may depend on your terminal configuration.)_

## Tutorials

These tutorials may help you get started with cursive:

* [Starting with cursive: (1/3)](https://github.com/gyscos/Cursive/tree/master/doc/tutorial_1.md)
* [Starting with cursive: (2/3)](https://github.com/gyscos/Cursive/tree/master/doc/tutorial_2.md)
* [Starting with cursive: (3/3)](https://github.com/gyscos/Cursive/tree/master/doc/tutorial_3.md)

## Third-party views

Here are a few crates implementing new views for you to use:

* [cursive_table_view](https://github.com/BonsaiDen/cursive_table_view)
* [cursive_calendar_view](https://github.com/BonsaiDen/cursive_calendar_view)
* [cursive_tree_view](https://github.com/BonsaiDen/cursive_tree_view)
* [cursive_hexview](https://github.com/hellow554/cursive_hexview)

## Goals

* **Ease of use.** Simple apps should be simple. Complex apps should be manageable.
* **Linux TTY Compatibility.** Colors may suffer, and UTF-8 may be too much, but most features *must* work properly on a Linux TTY.
* **Flexibility.** This library should be able to handle simple UI scripts, complex real-time applications, or even games.
    * In particular, it tries to have enough features to recreate these kind of tools:
        * [menuconfig](http://en.wikipedia.org/wiki/Menuconfig#/media/File:Linux_x86_3.10.0-rc2_Kernel_Configuration.png)
        * [nmtui](https://access.redhat.com/documentation/en-US/Red_Hat_Enterprise_Linux/7/html/Networking_Guide/sec-Configure_a_Network_Team_Using_the_Text_User_Interface_nmtui.html)

## Compatibility

First off, terminals are messy. A small set of features is standard, but beyond that, almost every terminal has its own implementation.

### Output

* **Colors**: the basic 8-colors palette should be broadly supported. User-defined colors is not supported in the raw linux TTY, but should work in most terminals, although it's still kinda experimental.
* **UTF-8**: Currently Cursive really expects a UTF-8 locale. It may eventually get patched to support window borders on other locales, but it's not a priority.
There is initial support for [wide characters](https://en.wikipedia.org/wiki/CJK_characters). [RTL](https://en.wikipedia.org/wiki/Right-to-left) support [is planned](https://github.com/gyscos/Cursive/issues/31), but still very early.

### Input

* The `key_codes` example can be a useful tool to see how the library reacts to various key presses.
* Keep in mind that if the terminal has shortcuts registered, they probably won't be transmitted to the app.
* UTF-8 input should work fine in a unicode-enabled terminal emulator, but raw linux TTY may be more capricious.

## [Contributing](CONTRIBUTING.md)

## Alternatives

See also [tui-rs](https://github.com/fdehau/tui-rs) - and a small [comparison page](https://github.com/gyscos/Cursive/wiki/Cursive-vs-tui%E2%80%90rs).
