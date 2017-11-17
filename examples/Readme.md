# Cursive Examples

Here are example programs using Cursive to help you getting familiar with the
various aspects of the library.

To run an example, use `cargo run --example EXAMPLE_NAME`.

## `hello_world`

Simplest example possible, it will show you the starting point of a basic
Cursive application.

## `dialog`

This example wraps the text in a `Dialog` view, showing the basic idea of view
composition.

## `lorem`

This example loads a large text file to show scrolling behaviour. It also
includes greek and japanese characters to show non-ascii support.

## `edit`

Here we have an `EditView` to get input from the user, and use that input in
the next view. It shows how to identify a view with an ID and refer to it
later.

## `mutation`

This example modifies the content of an existing view.

## `linear`

This example uses a `LinearView` to put multiple views side-by-side.

## `menubar`

Here we learn how to create a menubar at the top of the screen, and populate
it with static and dynamic entried.

## `logs`

This example defines a custom view to display asynchronous input from a
channel.

## `key_codes`

This example uses a custom view to print any input received. Can be used as a
debugging tool to see what input the application is receiving.

## `select`

This example uses a `SelectView` to have the user pick a city from a long list.

## `list_view`

This shows a use of a `ListView`, used to build simple forms.

## `text_area`

This example uses a `TextArea`, where the user can input a block of text.

## `theme`

This loads a theme file at runtime to change default colors.

## `theme_manual`

Instead of loading a theme file, this manually sets various theme settings.

## `terminal_default`

This example shows the effect of the `Color::TerminalDefault` setting.

## `colors`

This example draws a colorful square to show off true color support.

## `refcell_view`

Here we show how to access multiple views concurently through their IDs.

## `progress`

This shows how to send information from an asynchronous task (like a download
or slow computation) to update a progress bar.

## `radio`

This shows how to use `RadioGroup` and `RadioButton`.

## `slider`

This is a demonstration of the `SliderView`.
