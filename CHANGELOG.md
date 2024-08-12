# Changelog

## cursive-core 0.4.6

- Remove serde_yaml dependency (moved to dev-dependency)

## cursive-core 0.4.5

- Implement Resolvable for more types in the style module

## cursive-core 0.4.4

- Implement more standalone blueprints
- Panel and PaddedView blueprints now use `view` instead of `child`
- Implement Debug for `builder::Context`

## cursive-core 0.4.3

- Implement `Resolvable` for more types.
- Relax `Sync` bound on FnMut and FnOnce callbacks.

## cursive-core 0.4.2

- Raise `enumset` version in dependency to make sure `EnumSet::empty()` is const fn.

## cursive-core 0.4.1

- Add missing `Sync` bound on `View`
- Doc fixes

## cursive 0.21

### Breaking Changes

- Defaults to crossterm backend (instead of ncurses)
- Updates termion to 4.0
- Updates crossterm to 0.28.1
- Updates cursive-core to 0.4.0
- Updates ncurses to 6.0.1

### Improvements

- Removed unused (and unmaintainted) `term_size` dependency.
- Added a `status_bar` example.

### Bugfixes

- Crossterm backend: properly reset the color when de-initializing.

## cursive-core 0.4.0

### Breaking Changes

- The `View` now requires `Send + Sync`, to allow accessing or moving views between threads.
  This prevents using `Rc`/`RefCell`, and may require using `Arc`/`Mutex` instead.
  This should eventually open the way for more multi-threaded processing of the view tree.
- `theme::Style::effects` is now a map from `Effect` to `EffectStatus`.
- The `Backend` trait was changed:
    - `print_at` was split into `move_to` and `print`.
    - `print_at_rep` was removed.
- Some dependencies were updated:
    - toml was bumped from 0.5 to 0.8

### API updates

- The new experimental `builder` module (enabled via the `builder` feature) enables config-based view instanciation.
  View trees can be described in config files (yaml/json/...), and resolved, using parameters for interpolation.
- Added the `EditableText` family of palette styles.
- Added `Cursive::clear_all_global_callbacks()`.
- Improved `CursiveLogger`
- Added `Event::char(&self) -> Option<char>`
- Some functions are now callable in const context.
- Most of `cursive::theme` has moved to a new `cursive::style` module, with a re-export from `cursive::theme` for backward compatibility.
- Added `cursive::style::{Rgb, gradient}` for better gradient support.
- Added `GradientView`.
- Added `cursive::utils::markup::cursup` for a simple cursive-focused markup parser.
- Added `cursive::utils::markup::gradient` to decorate text with gradients.
- Made `cursive::theme::Theme::load_toml` public.
- `SelectView` can now use different decorators instead of `< >`.

## Bugfixes

- Fix shift+tab handling on termion.
- Fix possible panics with empty `MenuPopup`.

## Improvements

- The menubar now properly supports styled entries.
- The output to the backend is now buffered and delta-patched, resulting in improved performance for most backends.
- `owning_ref` was replaced with `parking_lot`
- `pulldown_cmark` was updated to 0.10
- `ansi-parser` was updated to 0.9
- Scrollable pages now scroll an entire page on left/right key presses.
- Fixed example links in Readme.md.

## cursive-core 0.3.7

### API updates

- Added "inactive highlight" property to SelectView.
- Added `{Theme, Palette}::{retro, terminal_default}()`.
- Added convenient method to create `Style`, similar to `ColorStyle`.
- Added `{ColorStyle, Style}::view()` with Primary/View colors.
- Added methods to create `RadioButton` using a key `&str`,
  rather than relying on shared `RadioGroup`.
- Many views now support using `StyledString` instead of just plain `String`.
    - Menu entries
    - Buttons
    - Dialog titles and buttons
    - Panel title
    - Radio button labels

### Bugfixes

- Fixed a focus update in `SelectView` that could result in no entry being selected.
- Fixed endless loop in `MenuPopup` (from the menubar for example) when all entries are disabled.

### Other Changes

- `Style::{highlight, highlight_inactive}` now rely on `Effect::Reverse`.
- Most styles have been changed to use `InheritParent` for their background.
    - `Layer` now explicitly uses `PaletteColor::View`.
- `Printer::print_styled` now takes `S: Into<SpannedStr>` rather than a `SpannedStr` directly.
  This lets it directly takes `&StyledString` as input.

## cursive-core 0.3.6

### API updates

- Add `ColorStyle::{map, zip_map}`
- Add `ColorStyle::invert`
- Add `impl From<BaseColor> for ColorType`

### Improvements

- Added more doc and doc tests to `ColorStyle`.
- Add a Minimum Supported Rust Version to Cargo.toml for a better error
  message on old toolchains.

### Bugfixes

- Fix the `immut3!` macro.
- Reset the running state when using non-default runners.
- Fix `ListView` behaviour with delimiters.
- Reset focus field when clearing `LinearLayout`.
- Fix scroll operation using outdated size if the child view was modified.

## cursive 0.20.0

### Breaking Changes

- Updates crossterm to 0.25.0

## cursive-syntect 0.1.0

- First release

## cursive-core 0.3.5

### Bugfixes

- Termion backend: properly revert terminal to blocking when exiting
  application.

### Improvements

- Added an ANSI color code parser
- Added some examples:
    - `advanced_user_data`
    - `ansi`
    - `theme_editor`
- Improved documentation for Printer

## cursive 0.19.0

### Breaking Changes

- Updates crossterm to 0.24.0

## cursive-core 0.3.4

### Improvements

- SelectView now caches its required size for improved performance.

### Bugfixes

- Fix focused state in `FixedLayoutView`.

## cursive-core 0.3.3

### Bugfixes

- Fix layout size inside `ScrollView`.

## cursive 0.18.0

### Breaking Changes

- Updated crossterm to 0.23.0

## cursive-core 0.3.2

### Bugfixes

- Fix focus for `SelectView::insert_item`.
- Fix scroll offset before the first layout call.
- Fix ResizedView to never give a larger size than available.

## cursive-core 0.3.1

### Bugfixes

- Fix title layout for Panel when the space is limited

## cursive-core 0.3.0, cursive 0.17.0

### API updates

- Add getters & other utility methods to Dialog
- Add enabled state to menu items
- Add a `reexports` module with re-exports of crates used in public API
- Add `ThemedView`
- Replace `Rect: From<T: Into<Vec2>>` with `Rect::from_point`
- Add `HideableView::visible`
- Add more control to focus changes, including `Event::FocusLost`
- Add `EventResult::with_cb_once`
- Add `Effect::Dim` (supported on some backends)
- Add `LinearLayout::clear`
- Add `backends::try_default`
- Add `StackView::layer_offset` and deprecate `StackView::offset`
- Add `Cursive::set_window_title` to change the terminal window title

### Breaking Changes

- Dependencies update:
    - Replaced `wasmer_enumset` with `enumset`
    - Replaced `chrono` with `time` for logger
- Removed a bunch of deprecated methods and types:
    - All `_id` methods that were replaced with `_name` equivalent
    - `BoxView`, `ViewBox`, `SizedView`, `Identifiable`, `Boxable`, `IdView`, `Selector::Id`
- Added `set_title` to the `Backend` trait
- Update dependencies:
    - crossterm to 0.22.1
    - pancurses to 0.17.0

### Bugfixes

- Fix an issue with focus for Dialog
- Fix `important_area` for ListView
- Do not shrink panels under the size required for the title
- Include wheel down event on legacy pancurses systems
- Use non-blocking IO on termion backend
- Fix `Align::bot_right`
- Fix delimiter handling in `ListView`
- Fix input handling in crossterm backend
- Fix focus issue with SelectView when in popup mode
- Updated internal dependencies:
    - enum-map to 2.0
    - pulldown-cmark to 0.9

## cursive 0.16.3

### API updates

- Implement `Borrow<Cursive>` for `CursiveRunnable`

## cursive-core 0.2.2, cursive 0.16.2

### API updates

- Add methods to turn a CursiveRunnable into a CursiveRunner.

## cursive 0.16.1

### Bugfixes

- Fix mouse input with crossterm backend.

## cursive-core 0.2.1

### Bugfixes

- Fix colors in menubar.

## cursive-core 0.2.0, cursive 0.16

### Breaking Changes

- Backends are now initialized when starting the event loop rather than when creating Cursive.
    - As a result initialization and runner functions have changed.
- `ColorStyle::color` is no longer an optional. Use `ColorType::InheritParent` if needed.
- Replaced `()` error types with some custom zero-sized types.

### API updates

- Add `ProgressBar::set_{min,max,range,counter,label}` for non-chained API.
- Derive Clone, Copy, Debug, PartialEq, Hash for more types.
- Add backend initializers using other files than /dev/tty for ncurses and termion.
- Add `CursiveRunner` to handle an event loop.
- `XY<T>` now implements `Default` for `T: Default`.
- `Style` now implements `FromIterator<&Style>` to merge multiple styles.
- `XY::stack_{horizontal,vertical}` are now `must_use`.
- `SpannedString` now implements `FromIterator<SpannedString>`.
- `view::ScrollBase` is now deprecated in favor of the `view::scroll` module.
- Add `Finder::call_on_all` and `Cursive::call_on_all_named` to call the same closure of
  several views with the same type and name.
- Add `SpannedString::remove_spans` to remove spans from a StyledString.
- Add `SpannedString::compact` to compact the source to only include span content.
- Add `SpannedString::trim(_{start, end})` to remove the unused prefix, suffix or both of the source.
- Add `SpannedString::spans(_raw)_attr_mut` to give mutable access to the attribute of the spans.
- Add `TextContent::with_content` to give mutable access to the `StyledString` of a `TextView`.
- Add `ColotyType::InheritParent` to carry over the front or back color from the parent.
- Add `Effect::Blink`.
- Add `Margins::zeros()`.
- Add `TextView::set_style`.
- Add `SpannedString`-shrinking methods.

### Improvements

- `ListView` now supports children taller than 1 row.
- Added an `autocomplete` example.
- Added a `stopwatch` example.
- `SpannedString` iterators are now double-ended and exact-sized.

### Bugfixes

- Fix scroll module when inner view size is close to available size.
- Fix text alignment for wrapped lines.
- Fix focus change with ScrollView.
- Fix possible crash with the BearLibTerminal backend.
- Dispatch `call_on_*` methods to all screens.
- Fix potential issue with `setlocale` in ncurses backend on BSD.
- Fix handling of multi-bytes newlines characters.

### Doc

- Improve documentation for themes.

## cursive-core 0.1.1

### API updates

- Add `Dialog::into_content`.
- Add `Callback::from_fn_once` and `once1!` macro to wrap a `FnOnce` in a `FnMut`.
- Add `FixedLayoutView` with manual placement of child views.
- Add `OnLayoutView` to override `View::Layout`
- Add `Cursive::{dump, restore}` to save and load the global state.
- Add `NamedView::{name, set_name}` to retrieve or replace the given name.
- Add `LinearLayout::find_child_with_name`.
- Add `ScrollView::on_scroll` callback.
- Add `once1!` macro to turn a `FnOnce` into `FnMut`.
- Implement `Default` for some wrapper views where the child is `Default`.

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

- Update `enum-map` from 0.5 to 0.6

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
  being laid out.
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
