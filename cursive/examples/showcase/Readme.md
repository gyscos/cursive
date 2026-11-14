# Showcase

A fake "mission control" dashboard, meant to show what a complete cursive
application can look like.

```
cargo run --release --example showcase
cargo run --release --example showcase -- --demo   # self-driving
```

It needs a terminal with true color support, and a font with braille and block
characters.

What to look for:

* [`main.rs`](./main.rs): the layout, built from standard views (menubar,
  `LinearLayout`, `TabView` with its bar in the sidebar, `ScrollView`, `Dialog`,
  `ListView` forms, `ProgressBar`...), and the callbacks tying them together.
  Also shows how a background thread feeds the interface, through shared
  state, `TextContent` and `cb_sink`. The shortcuts of the dashboard are an
  `OnEventView` around it, so they do not fire while a dialog is open; the demo
  mode is a script given to `Cursive::play_events`.
* The data views in `main.rs`: `Chart` (an area and a line plot), `Heatmap` and
  `Sparkline`, all reading `Series` handles that the simulation thread pushes
  to. Their colors are `Fill` gradients made of the theme's custom colors, so
  they follow theme changes. The boxes around them are standard `Panel`s: the
  rounded, dim border comes from the theme.
* [`widgets.rs`](./widgets.rs): the only custom view (the header), and the
  panels made of standard views: stat tiles (a `Row` of live `TextView`s above a
  `Sparkline`), `Row`s with live `TextView` labels and values for the regions
  (the dot follows the latency) and live values for the health panel, plus `Gauge`s
  reading the simulation through closures for the services and the health
  panel. Threshold colors are one `Fill::steps` each, used for the bars and,
  through `Fill::color_for`, for the text. The custom view is a `View` with
  little more than a `draw` method, using `Printer::print_styled_aligned` and
  `Fill`s built from the theme's colors.
* [`themes.rs`](./themes.rs): color schemes turned into cursive `Theme`s, which
  can be swapped while the application runs (press `t`).
* [`sim.rs`](./sim.rs): the fake data source. Apart from the `Series` it
  pushes samples to, nothing cursive-specific there.

[`record.sh`](./record.sh) records the demo mode into the animation used in the main
readme (requires `asciinema`, `agg` and `ffmpeg`).
