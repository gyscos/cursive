# Cursive Examples

Here are example programs using Cursive to help you getting familiar with the
various aspects of the library.

To run an example, use `cargo run --bin EXAMPLE_NAME`.

To use a specific cursive backend, you can do, for example:

```
cargo run --bin EXAMPLE_NAME --features cursive/crossterm-backend
```

## [`hello_world`](src/bin/hello_world.rs)

Simplest example possible, it will show you the starting point of a basic
Cursive application.

## [`dialog`](src/bin/dialog.rs)

This example wraps the text in a `Dialog` view, showing the basic idea of view
composition.

## [`lorem`](src/bin/lorem.rs)

This example loads a large text file to show scrolling behaviour. It also
includes greek and japanese characters to show non-ascii support.

## [`edit`](src/bin/edit.rs)

Here we have an `EditView` to get input from the user, and use that input in
the next view. It shows how to identify a view with an name and refer to it
later.

## [`mutation`](src/bin/mutation.rs)

This example modifies the content of an existing view.

## [`linear`](src/bin/linear.rs)

This example uses a `LinearView` to put multiple views side-by-side.

## [`menubar`](src/bin/menubar.rs)

Here we learn how to create a menubar at the top of the screen, and populate
it with static and dynamic entried.

## [`logs`](src/bin/logs.rs)

This example defines a custom view to display asynchronous input from a
channel.

## [`key_codes`](src/bin/key_codes.rs)

This example uses a custom view to print any input received. Can be used as a
debugging tool to see what input the application is receiving.

## [`select`](src/bin/select.rs)

This example uses a `SelectView` to have the user pick a city from a long list.

## [`list_view`](src/bin/list_view.rs)

This shows a use of a `ListView`, used to build simple forms.

## [`text_area`](src/bin/text_area.rs)

This example uses a `TextArea`, where the user can input a block of text.

## [`markup`](src/bin/markup.rs)

This example prints a text with markup decorations.

## [`theme`](src/bin/theme.rs)

This loads a theme file at runtime to change default colors.

## [`theme_manual`](src/bin/theme_manual.rs)

Instead of loading a theme file, this manually sets various theme settings.

## [`terminal_default`](src/bin/terminal_default.rs)

This example shows the effect of the `Color::TerminalDefault` setting.

## [`colors`](src/bin/colors.rs)

This example draws a colorful square to show off true color support.

## [`refcell_view`](src/bin/refcell_view.rs)

Here we show how to access multiple views concurently through their name.

## [`progress`](src/bin/progress.rs)

This shows how to send information from an asynchronous task (like a download
or slow computation) to update a progress bar.

## [`radio`](src/bin/radio.rs)

This shows how to use `RadioGroup` and `RadioButton`.

## [`slider`](src/bin/slider.rs)

This is a demonstration of the `SliderView`.

## [`mines`](src/bin/mines) (**Work in progress**)

A larger example showing an implementation of minesweeper.
