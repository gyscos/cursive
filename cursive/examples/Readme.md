# Cursive Examples

Here are example programs using Cursive to help you getting familiar with the
various aspects of the library.

To run an example, use `cargo run --example EXAMPLE_NAME`.

To use a specific cursive backend, you can do, for example:

```
cargo run --example EXAMPLE_NAME --features crossterm-backend --no-default-features 
```

## [`hello_world`](./hello_world.rs)

Simplest example possible, it will show you the starting point of a basic
Cursive application.

## [`dialog`](./dialog.rs)

This example wraps the text in a `Dialog` view, showing the basic idea of view
composition.

## [`lorem`](./lorem.rs)

This example loads a large text file to show scrolling behaviour. It also
includes greek and japanese characters to show non-ascii support.

## [`edit`](./edit.rs)

Here we have an `EditView` to get input from the user, and use that input in
the next view. It shows how to identify a view with an name and refer to it
later.

## [`mutation`](./mutation.rs)

This example modifies the content of an existing view.

## [`linear`](./linear.rs)

This example uses a `LinearView` to put multiple views side-by-side.

## [`menubar`](./menubar.rs)

Here we learn how to create a menubar at the top of the screen, and populate
it with static and dynamic entried.

## [`menubar_styles`](./menubar_styles.rs)

Example of using StyledString in menubar lables.

## [`logs`](./logs.rs)

This example defines a custom view to display asynchronous input from a
channel.

## [`key_codes`](./key_codes.rs)

This example uses a custom view to print any input received. Can be used as a
debugging tool to see what input the application is receiving.

## [`select`](./select.rs)

This example uses a `SelectView` to have the user pick a city from a long list.

## [`list_view`](./list_view.rs)

This shows a use of a `ListView`, used to build simple forms.

## [`text_area`](./text_area.rs)

This example uses a `TextArea`, where the user can input a block of text.

## [`markup`](./markup.rs)

This example prints a text with markup decorations.

## [`theme`](./theme.rs)

This loads a theme file at runtime to change default colors.

## [`theme_manual`](./theme_manual.rs)

Instead of loading a theme file, this manually sets various theme settings.

## [`terminal_default`](./terminal_default.rs)

This example shows the effect of the `Color::TerminalDefault` setting.

## [`colors`](./colors.rs)

This example draws a colorful square to show off true color support.

## [`colored_text`](./colored_text.rs)

This example showcasing various methods to color and remove text styles, highlighting the limitations of Crossterm's raw ANSI output.

## [`refcell_view`](./refcell_view.rs)

Here we show how to access multiple views concurrently through their name.

## [`progress`](./progress.rs)

This shows how to send information from an asynchronous task (like a download
or slow computation) to update a progress bar.

## [`radio`](./radio.rs)

This shows how to use `RadioGroup` and `RadioButton`.

## [`slider`](./slider.rs)

This is a demonstration of the `SliderView`.

## [`mines`](./mines) (**Work in progress**)

A larger example showing an implementation of minesweeper.

## [`window_title`](./window_title.rs)

This shows how to change the terminal window title.
