# Changelog

## (Next version) cursive-core=0.1.1

### API updates

- Add `Dialog::into_content`.
- Add `Callback::from_fn_once` and `once1!` macro to wrap a `FnOnce` in a `FnMut`.
- Add `FixedLayoutView` with manual placement of child views.
- Add `OnLayoutView` to override `View::Layout`

### Bugfixes

- More hygienic `wrap_impl!` macro using fully-qualified paths.
- Fixed `LinearLayout` giving children incorrect sizes.
- More accurate "important area" for `ShadowView` and `PaddedView`.
- Fix potential panic when calling `TextArea::set_cursor` before its first layout phase.
- Disabled views no longer accept input if they are still in focus.

## 0.15.0

### Breaking changes

- Split library into a backend-agnostic `cursive-core` and a user-facing `cursive`.
- `Cursive::default` now needs the `cursive::CursiveExt` trait to be in scope.
- Update dependencies:
    - crossterm to 0.17.
    - enumset to 1.0
    - ahash to 0.3
    - pulldown-cmark to 0.7
- Add `PaletteColor::HighlightText`
- `AnyCb` now takes a `&mut dyn View` rather than a `&mut dyn Any`.

### API updates

- Added `cursive::{default,ncurses,pancurses,termion,crossterm,blt,dummy}` functions.
- Add `Cursive::debug_name`
- Add `ScreensView` to move some code away from the `Cursive` root
    - Reworked global callbacks configuration
    - Ctrl-C can be rewired to no longer exit the application
- Add `SelectView::(try_)iter_mut()`
- `Dialog::{test, info}` now accept `StyledString` as input
- Add missing functions to Checkbox re: enabled state

### Bugfixes

- Fix Ctrl-Z binding for ncurses
- Fix potential crash with empty `SelectView`
- Add `toml` and `markdown` features to docs.rs

## 0.14.0

### Breaking changes

- `cursive::event::AnyCb` changed from `Box<...>` to `&mut ...`, so users of
  `View::call_on_any` no longer need to box their closures.
- Remove `BoxView::squishable`.
- Update crossterm to 0.14.
- Removed `From` implementations for `Margins`. Use `Margins::lrtb` and the like instead.
    - Or `Dialog::padding_lrtb`.
- Renamed multiple types (old names are still re-exported, but deprecated):
    - `BoxView` -> `ResizedView`
    - `ViewBox` -> `BoxedView`
    - `SizedView` -> `LastSizeView`
    - `Identifiable` -> `Nameable`
    - `Boxable` -> `Resizable`
    - `IdView` -> `NamedView`
    - `Selector::Id` -> `Selector::Name`
    - `with_id` -> `with_name`
    - `call_on_id` -> `call_on_name`
    - `find_id` -> `find_name`
    - `focus_id` -> `focus_name`

### API updates

- `SelectView::{item, with_all}` now accept `S: Into<StyledString>` for colored labels.
- Add `ScrollView::scroll_to_important_area`.
- Add `LinearLayout::set_focus_index`.
- Add `XY::{sum, product}`.
- `view::scroll` is now a public module.
- Add `Cursive::process_events` and `Cursive::post_events`.
    - This gives users finer control than `Cursive::step`.
- `Layer` now has a `color` option.
- `LinearLayout` can now directly add boxed views without re-boxing.
- Add inner getters to `EnableableView`.
- Add `PaddedView::get_inner(_mut)`.
- Add a bunch of constructors for `Margins`.
- Add `Dialog::padding_lrtb`
- Add `Dialog::set_padding*`
- Add `PaddedView::lrtb`

### Improvements

- Changed the default color for `TitleSecondary` from yellow to light blue.
- Changed the default color for `Tertiary` from grey to white.
- Reduced dependencies (`toml` is now optional, removed `hashbrown`).
- `Cursive::default()` now fallbacks do dummy backend if no other is available.

### Bugfixes

- Fixed `ScrollView::show_scrollbars()`.
- Correctly update the offset for `ScrollView` after focus change.
- Fixed layout for `BoxView` with some size constraints.
- On Windows, do not print unix-specific character during initialization.
- Fix out-of-bounds access for some mouse events in `MenuPopup`

## 0.13.0

### Breaking changes

- Update `enum-map` fron 0.5 to 0.6

### API updates

- Add `Effect::Strikethrough` (not supported on ncurses)
- Add `ListView::remove_child`
- Replace `xursive::CbFunc` with `Box<FnOnce>`
- Add `ScrollView::{inner_size, is_as_{bottom, top, left, right} }`
- Add getters for current value in `SliderView`
- More fields made public in `cursive::logger`
- Add a "puppet" backend for testing and instrumentation

### Improvements

- Performance improvements for the crossterm backend

### Bugfixes

- Fix a possible panic when a TextView is updated asynchronously while it's
  being layed out.
- Fixed weird behaviour of `SizeConstraint::Full` with `ScrollView`.

## 0.12.0

### Breaking changes

- Updated `enumset` from 0.3 to 0.4

### API updates

- Add `Cursive::take_user_data`, replaces the current user data with `()`.
- Add `SliderView::{get_value, get_max_value}`.

### Improvements

- `DebugConsole` now has horizontal scrolling enabled.
- `pancurses` backend now correctly recognizes the "Enter" key from the numpad
  as "Enter".

## 0.11.2

### API updates

- Bring back `Cursive::set_fps` for <30Hz refresh rates.
- Add `Cursive::backend_name` to get the name of the current backend.
- Add a new backend based on the crossterm library.
- Add direct downcast methods to `dyn AnyView`
- Add sort methods to `SelectView`

### Improvements

- Improved printer performance with styled spans.

## 0.11.1

### API updates

- Added manual scrolling methods to `view::scroll::Core`:
    - `keep_in_view`, `scroll_to`, `scroll_to_x`, `scroll_to_y`
    Note: the `view::scroll` module is hidden behind an experimental
    feature `unstable_scroll`.

### Improvements

- Improved printer performance (thanks to @chrisvest).

### Bugfixes

- Fixed `MenuPopup` borders near delimiters.

## 0.11.0

### Breaking changes

- `Cursive::{ncurses, pancurses, termion}` now return
  `io::Result<Self>` instead of panicking. `Cursive::default()` still unwraps.
  - Also added `Cursive::try_new` for failible backends.
- Replaced `set_fps(i32)` with `set_autorefresh(bool)`
- `Finder::find_id()` is renamed to `call_on_id()`, and a proper
  `find_id()` was added instead.
- Updated the Backend trait for a simpler input system
- Updated to Rust 2018 edition (now requires rustc > 1.31)
- `Cursive::clear()` now takes `&mut self`

### API updates

- Add a logging implementation (`logger::init()`) and a `DebugConsole`
  (`cursive::toggle_debug_console()`)
- Add user-data to Cursive.
    - `Cursive::set_user_data()` can store some user-defined data structure.
    - `Cursive::user_data()` and `Cursive::with_user_data()` can be used to
      access the data.
- Add `StackView::remove_layer()`
- Add `CircularFocus` view (and bring proper circular focus to dialogs)
- Add `HideableView::is_visible()`
- Add `type CbSink = Sender<Box<CbFunc>>` as an alias for the return type of
  `Cursive::cb_sink()`
- Add `LinearLayout::{insert_child, swap_children, set_weight}` for more
  in-place modifications.
- Add `Printer::{cropped_centered,shrinked_centered}`

### Improvements

- Updated termion backend to use direct /dev/tty access for improved performance.
- Enabled raw mode for ncurses and pancurses. Among other improvements, this
  lets applications receive Ctrl+S and Ctrl+Q events.

### Bugfixes

- Fixed overflow check for titles in `Dialog` and `Panel`

## 0.10.0

### New features

- Add `EventTrigger` and update `OnEventView` to use it.
    - Breaking change: "inner" callbacks for OnEventView now take the event as
      extra argument.
- Add `Printer::enabled` and `EnableableView` to disable whole subtrees.
- Add `RadioGroup::on_change` to set a callback on selection change.
- `SelectView` now uses `StyledString` to display colored text.
- Add `PaddedView` to add padding to any view.
- Update dependencies
    - Breaking change: crossbeam-channel was updated, and using `send()` now
      returns a `Result`.

### Bugfixes

- Fix mouse events on Ubuntu

### Doc

- Added examples to most utility types (`XY`, `Vec2`, ...)

## 0.9.2

### New features

- Add an optional title to `Panel`
- Add `immut1!`, `immut2!` and `immut3!` macros to wrap a `FnMut` in `Fn`
- SelectView: autojump is now opt-in (jump to an element after a letter is pressed)

### Bugfixes

- Fix possible crash with `ListView` and `SelectView` in very small spaces
- Fix termion backend compilation on non-unix platforms

## 0.9.1

### New features

- Add `Cursive::on_event` to send simulated events.
- Add `EventResult::and` to combine callbacks.
- Allow custom color in `ProgressBar`.

### Bugfixes

- LinearLayout:
    - Better geometry computation with constrained size
    - Fixed cache invalidation
    - Fix possible panic when removing children
- ScrollView:
    - Fix possible panic with full-height scrollbar
    - Fix possible panic with empty content
    - Fix cache
- Fix menubar focus after action

## Doc

- Fix Readme and examples with `Cursive::default()`

## 0.9.0

### New features

- Cursive now supports third-party backends
- Add generic `ScrollView` wrapper. Removes internal scrolling behaviour from
  `TextView`.
- Callbacks sent through `Cursive::cb_sink()` are now processed instantly,
  without the need for `set_fps`.
- Make backend a dynamic choice
    - User must select a backend in `Cursive::new`
    - 3rd party libraries do not need to play with backend features anymore
- Move from `chan` and `chan-signals` to `crossbeam-channel` and `signal-hook`
- Batch-process events for higher performance
- Add `StackView::find_layer_from_id`
- Add `SelectView::insert_item`
- Add `TextArea::{enable, disable}`
- Reworked `AnyView`
- `SelectView`: Fix mouse events
- Return callbacks from manual control methods
    - `SelectView::{set_selection, select_up, select_down, remove_item}`
    - `EditView::{set_content, insert, remove}`
- Add `rect::Rect`
- Add `HideableView`

### Changes

- Replaced `Cursive::new()` with `Cursive::default()`
- Renamed `Vec4` to `Margins`
- `Callbacks` cannot be created from functions that return a value
    - The returned value used to be completely ignored
- `AnyView` does not extend `View` anymore (instead, `View` extends `AnyView`)
    - If you were using `AnyView` before, you probably need to replace it with `View`
- Scrolling is now added to a view with `.scrollable()`
- `cb_sink` is now a `crossbeam_channel::Sender` instead of `chan::Sender`
- `SelectView::selection` now returns an `Option<Rc<T>>` instead of just `Rc<T>`.
   It will return `None` if the `SelectView` is empty.


## 0.8.1

### New features

- Add `Cursive::clear_global_callbacks`

### Bugfixes

- Fix non-ASCII input with pancurses backend
- Fix `StackView::move_layer`
- Fix layout computation for `SelectView`
- Remove unused `maplit` dependency for termion and blt backends

## 0.8.0

### New features

- Style (breaking change):
    - Added support for bold/italic/underlined text
    - Added `StyledString` for markup text
    - Refactored line-break module
- Colors (breaking change):
    - Added ColorStyle and PaletteColor for more flexible colored text
- Buttons:
    - Added `Dialog::buttons` to iterate on buttons
    - Added `Button::set_label` and `Button::label`
- TextView:
    - Added TextContent, a way to separate model and view for TextView
    - Added manual scrolling methods
- Allow multiple global callbacks per event
- Allow buttons and delimiters in top-level menubar
- StackView:
    - Added `StackView::move_layer` to re-order layers
    - `StackView::pop_layer` now returns the pop'ed view
    - Added `StackView::reposition_layer` to move a layer around
- Dialog: added `Dialog::focus(&self)`
- SelectView: added `SelectView::selected`
- `Cursive::cb_sink` now accepts `FnOnce` (previously `Fn` only)

### Bugfixes

- Fix a bug in `TextArea::set_content`
- Fix `Color::from_256colors` for grayscale colors
- Fix resize detection on windows
- Fix possible panic with weird input on pancurses
- Fix possible panic in ListView layout

### Doc

- Add per-distributions instructions to install ncurses
- Improved comments in examples
- Improve doc for `Cursive::find_id`
- Improve doc for `Identifiable::with_id`
- Include Changelog
