# Cursive

[![crates.io](https://img.shields.io/crates/v/cursive.svg)](https://crates.io/crates/cursive)
[![Rust](https://github.com/gyscos/cursive/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/gyscos/cursive/actions/workflows/rust.yml)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Gitter chat](https://badges.gitter.im/gyscos/cursive.png)](https://gitter.im/cursive-rs/cursive)


Cursive is a TUI (Text User Interface) library for rust. It uses the [`crossterm`] backend by default, but [other backends are available](https://github.com/gyscos/cursive/wiki/Backends).

It allows you to build rich user interfaces for terminal applications.

[`crossterm`]: https://github.com/crossterm-rs/crossterm

# [Documentation](http://docs.rs/cursive)

It is designed to be safe and easy to use:

```toml
[dependencies]
cursive = "0.21"
```

Or to use the latest git version:

```toml
[dependencies]
cursive = { git = "https://github.com/gyscos/cursive" }
```

```rust,no_run
use cursive::views::{Dialog, TextView};

fn main() {
    // Creates the cursive root - required for every application.
    let mut siv = cursive::default();

    // Creates a dialog with a single "Quit" button
    siv.add_layer(Dialog::around(TextView::new("Hello Dialog!"))
                         .title("Cursive")
                         .button("Quit", |s| s.quit()));

    // Starts the event loop.
    siv.run();
}
```

[![Cursive dialog example](https://raw.githubusercontent.com/gyscos/cursive/main/doc/cursive_example.png)](https://github.com/gyscos/cursive/tree/main/cursive/examples/dialog.rs)

Check out the other [examples](https://github.com/gyscos/cursive/tree/main/cursive/examples) to get these results, and more:

<div>
<a href="https://github.com/gyscos/cursive/blob/main/cursive/examples/lorem.rs"><img src="https://imgur.com/hW9M9MV.png" alt="lorem.rs example", width="48%" /></a>
<a href="https://github.com/gyscos/cursive/blob/main/cursive/examples/menubar.rs"><img src="https://imgur.com/xx3lZPz.png" alt="menubar.rs example", width="48%" /></a>
<a href="https://github.com/gyscos/cursive/blob/main/cursive/examples/select.rs"><img src="https://imgur.com/couty0n.png" alt="select.rs example", width="48%" /></a>
<a href="https://github.com/gyscos/cursive/blob/main/cursive/examples/mines/"><img src="https://imgur.com/vNteYyy.png" alt="mines example", width="48%" /></a>
<a href="https://github.com/gyscos/cursive/blob/main/cursive/examples/theme_manual.rs"><img src="https://i.imgur.com/I9V5KRi.png" alt="theme_manual.rs example", width="48%" /></a>
<a href="https://github.com/gyscos/cursive/blob/main/cursive-syntect/examples/parse.rs"><img src="https://i.imgur.com/a8bfe5s.png" alt="syntect example" width="48%" /></a>
</div>

_(Colors may depend on your terminal configuration.)_

## Tutorials

These tutorials may help you get started with cursive:

* [Starting with cursive: (1/3)](https://github.com/gyscos/cursive/tree/main/doc/tutorial_1.md)
* [Starting with cursive: (2/3)](https://github.com/gyscos/cursive/tree/main/doc/tutorial_2.md)
* [Starting with cursive: (3/3)](https://github.com/gyscos/cursive/tree/main/doc/tutorial_3.md)

## Third-party views

Here are a few crates implementing new views for you to use:

* [cursive-aligned-view](https://github.com/deinstapel/cursive-aligned-view): A view wrapper for gyscos/cursive views which aligns child views.
* [cursive-async-view](https://github.com/deinstapel/cursive-async-view): A loading-screen wrapper.
* [cursive-flexi-logger-view](https://github.com/deinstapel/cursive-flexi-logger-view): An alternative debug view using `emabee/flexi_logger`.
* [cursive-markup](https://sr.ht/~ireas/cursive-markup-rs): A view that renders HTML or other markup.
* [cursive-multiplex](https://github.com/deinstapel/cursive-multiplex): A tmux like multiplexer.
* [cursive-spinner-view](https://github.com/otov4its/cursive-spinner-view): A spinner view.
* [cursive-tabs](https://github.com/deinstapel/cursive-tabs): Tabs.
* [cursive_calendar_view](https://github.com/BonsaiDen/cursive_calendar_view): A basic calendar view implementation.
* [cursive_hexview](https://github.com/hellow554/cursive_hexview): A simple hexview.
* [cursive_table_view](https://github.com/BonsaiDen/cursive_table_view): A basic table view component.
* [cursive_tree_view](https://github.com/BonsaiDen/cursive_tree_view): A tree view implementation.
* [cursive-hjkl](https://github.com/gamma-delta/cursive-hjkl): Wraps any view to use Vim-like `hjkl` controls.

## Showcases

Here are some cool applications using cursive:

* [RustyChat](https://github.com/SambaDialloB/RustyChat): Chat client made using Rust and Cursive.
* [checkline](https://github.com/sixarm/checkline-rust-crate): Checkbox line picker from stdin to stdout.
* [clock-cli](https://github.com/TianyiShi2001/clock-cli-rs): A clock with stopwatch and countdown timer functionalities.
* [fui](https://github.com/xliiv/fui): Add CLI & form interface to your program.
* [game2048-rs](https://github.com/genieCS/game2048-rs): a tui game2048 using Rust and cursive.
* [git-branchless](https://github.com/arxanas/git-branchless): Branchless workflow for Git.
* [grin-tui](https://github.com/mimblewimble/grin): Minimal implementation of the MimbleWimble protocol.
* [kakikun](https://github.com/file-acomplaint/kakikun): A paint and ASCII art application for the terminal.
* [launchk](https://github.com/mach-kernel/launchk): Manage launchd agents and daemons on macOS.
* [markline](https://github.com/sixarm/markline): Marker-based line picker from stdin to stdout.
* [mythra](https://github.com/deven96/mythra): CLI to search for music.
* [ncspot](https://github.com/hrkfdn/ncspot): Cross-platform ncurses Spotify client.
* [rbmenu-tui](https://github.com/DevHyperCoder/rbmenu-tui): A TUI for bookmark management.
* [retris](https://github.com/genieCS/retris): A simple implementation of the classic tetris game.
* [ripasso](https://github.com/cortex/ripasso): A simple password manager written in Rust.
* [rusty-man](https://sr.ht/~ireas/rusty-man): Browse rustdoc documentation.
* [saci-rs](https://gitlab.com/ihercowitz/saci-rs): Simple API Client Interface.
* [so](https://github.com/samtay/so): A terminal interface for Stack Overflow.
* [sudoku-tui](https://github.com/TianyiShi2001/sudoku-tui): Play sudoku on the command line.
* [tap](https://github.com/timdubbins/tap): An audio player for the terminal with fuzzy finder.
* [ttyloop](https://github.com/gamma-delta/ttyloop): Clone of the mobile game Loop.
* [wiki-tui](https://github.com/Builditluc/wiki-tui): A simple and easy to use Wikipedia Text User Interface
* [glues](https://github.com/gluesql/glues): A simple note-taking app with Git, CSV, and JSON support

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
There is initial support for [wide characters](https://en.wikipedia.org/wiki/CJK_characters). [RTL](https://en.wikipedia.org/wiki/Right-to-left) support [is planned](https://github.com/gyscos/cursive/issues/31), but still very early.

### Input

* The `key_codes` example can be a useful tool to see how the library reacts to various key presses.
* Keep in mind that if the terminal has shortcuts registered, they probably won't be transmitted to the app.
* UTF-8 input should work fine in a unicode-enabled terminal emulator, but raw linux TTY may be more capricious.

## [Contributing](CONTRIBUTING.md)
## Alternatives

See also [ratatui](https://github.com/ratatui-org/ratatui) - and a small [comparison page](https://github.com/gyscos/cursive/wiki/Cursive-vs-tui%E2%80%90rs).
