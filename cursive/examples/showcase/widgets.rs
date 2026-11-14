//! The title bar, and the tiles and panels of the dashboard.
//!
//! `Header` is the only custom view: a plain `View` implementation, which needs little more
//! than a `draw` method. It re-reads the shared simulation state every time it is drawn.
//! Colors are theme keys (see `themes.rs`), used directly in styles and `Fill`s: nothing is
//! resolved by hand.
//!
//! The charts are not here: `main.rs` builds them from the standard `Chart`, `Sparkline` and
//! `Heatmap` views. `stat_tile`, `health`, `service_list` and `region_list` are compositions of
//! standard views (`Row`, `Gauge`, `Sparkline`, live `TextView`s) reading the simulation
//! through closures.
//!
//! Thresholds are `Fill::steps`: the same `Fill` colors the bars, and gives the color of the
//! matching text through `Fill::color_for`.

use cursive::align::HAlign;
use cursive::style::chart::Fill;
use cursive::style::{ColorStyle, ColorType, Effect, Style};
use cursive::utils::format_compact;
use cursive::utils::markup::StyledString;
use cursive::view::{Resizable, View};
use cursive::views::{
    BarStyle, DummyView, Gauge, LinearLayout, Row, Sparkline, TextView, ValueRange,
};
use cursive::{Printer, Vec2};

use crate::sim::{Metric, Shared};
use crate::themes;

/// Green below `warn`, yellow below `err`, red above, for values from 0 to `max`.
///
/// A `Fill` works on fractions of a range, so values are divided by `max` first.
#[derive(Clone)]
struct Status {
    fill: Fill,
    max: f32,
}

impl Status {
    fn new(warn: f32, err: f32, max: f32) -> Self {
        Status {
            fill: Fill::steps([
                (0.0, &themes::OK),
                (warn / max, &themes::WARN),
                (err / max, &themes::ERR),
            ]),
            max,
        }
    }

    /// The color of the given value, for text.
    fn color(&self, value: f32) -> ColorType {
        self.fill
            .color_for(value / self.max)
            .expect("steps always give a color")
    }
}

/// Bold text in the given color.
fn bold(color: impl Into<ColorType>) -> Style {
    Style::from(ColorStyle::front(color)).combine(Effect::Bold)
}

// ---------------------------------------------------------------------------

/// The title bar: a gradient logo on the left, the clock on the right.
pub struct Header {
    shared: Shared,
}

impl Header {
    pub fn new(shared: &Shared) -> Self {
        Header {
            shared: shared.clone(),
        }
    }
}

impl View for Header {
    fn draw(&self, printer: &Printer) {
        let sim = self.shared.lock().unwrap();
        let width = printer.size.x;

        let title = "◆ CURSIVE MISSION CONTROL";
        let len = title.chars().count();
        printer.with_effect(Effect::Bold, |printer| {
            printer.print_filled(
                (1, 0),
                title,
                &Fill::horizontal([&themes::ACCENT, &themes::ACCENT2]),
            );
        });
        let subtitle = "prod-1 · 16 nodes · 8 services";
        if width > len + subtitle.chars().count() + 28 {
            printer.with_color(ColorStyle::front(&themes::DIM), |printer| {
                printer.print((len + 3, 0), subtitle);
            });
        }

        // Right side: state, then clock.
        let (label, color) = if sim.paused {
            ("‖ PAUSED", &themes::WARN)
        } else if sim.incident > 0.15 {
            ("▲ INCIDENT", &themes::ERR)
        } else if sim.tick / 4 % 2 == 0 {
            ("● LIVE", &themes::OK)
        } else {
            ("○ LIVE", &themes::OK)
        };
        let mut right = StyledString::styled(label, bold(color));
        right.append_plain(format!("  {}", sim.clock()));
        printer.print_styled_aligned((0, 0), width.saturating_sub(1), HAlign::Right, &right);
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        Vec2::new(constraint.x, 1)
    }
}

// ---------------------------------------------------------------------------

/// One big number, its recent trend, and a sparkline of its history.
///
/// A `Row` puts the number (label) and the trend (value) on one line: the number takes its
/// natural width first, so a narrow tile crops the trend, never the number. The trend is a
/// one-line wrapping `TextView` (made of non-breaking spaces): unlike a `no_wrap` one, it does not
/// ask for more width than it is offered, so the tiles share the width evenly. The sparkline
/// takes the rest of the height.
pub fn stat_tile(shared: &Shared, metric: Metric) -> impl View + use<> {
    // The series is a handle: it keeps showing new samples, and the tile
    // never needs to lock the simulation again.
    let series = shared.lock().unwrap().metric(metric).clone();

    // Percentages use their natural scale. The other metrics fit the visible
    // samples, with a margin on top and a minimum scale (1000 / 2 plus the margin).
    let (fill, range) = match metric {
        Metric::Cpu | Metric::Memory => (severity_fill(), ValueRange::from(0.0..=100.0)),
        Metric::Requests => (
            Fill::horizontal([&themes::ACCENT2, &themes::ACCENT]),
            ValueRange::auto().include(0.0).cover(1.0).margin(0.1),
        ),
        Metric::Errors => (
            severity_fill(),
            ValueRange::auto().include(0.0).cover(2.0).margin(0.1),
        ),
    };
    let status = match metric {
        Metric::Errors => Status::new(1.0, 2.5, 5.0),
        _ => Status::new(70.0, 85.0, 100.0),
    };

    let value = {
        let series = series.clone();
        TextView::live(move || {
            let value = series.last().unwrap_or(0.0) as f32;
            let (text, color) = match metric {
                Metric::Cpu | Metric::Memory => (format!("{value:.0}%"), status.color(value)),
                Metric::Requests => (
                    format!("{} req/s", format_compact(value.into())),
                    ColorType::from(&themes::ACCENT),
                ),
                Metric::Errors => (format!("{value:.2}%"), status.color(value)),
            };
            StyledString::styled(text, bold(color))
        })
        .no_wrap()
    };

    // How did it change recently?
    let trend = {
        let series = series.clone();
        TextView::live(move || {
            let delta = series.last().unwrap_or(0.0) as f32 - series.ago(40).unwrap_or(0.0) as f32;
            let text = if delta.abs() < 0.005 {
                "─ steady".to_string()
            } else if delta > 0.0 {
                format!("▲ {}", human_delta(delta))
            } else {
                format!("▼ {}", human_delta(-delta))
            };
            // Non-breaking spaces: the trend is one word, cut at the edge rather than wrapped.
            StyledString::styled(text.replace(' ', "\u{a0}"), ColorStyle::front(&themes::DIM))
        })
        .h_align(HAlign::Right)
        .fixed_height(1)
    };

    LinearLayout::vertical()
        .child(Row::from_views(value, trend).gap(1))
        .child(
            Sparkline::new(&series)
                .fill(fill)
                .range(range)
                .full_width()
                .full_height(),
        )
}

/// Goes through the "all good", "careful" and "bad" colors.
fn severity_fill() -> Fill {
    Fill::by_value([&themes::OK, &themes::OK, &themes::WARN, &themes::ERR])
}

/// Like `format_compact`, but always with two decimals below 10.
fn human_delta(delta: f32) -> String {
    if delta >= 1000.0 {
        format!("{:.1}k", delta / 1000.0)
    } else if delta >= 10.0 {
        format!("{delta:.0}")
    } else {
        format!("{delta:.2}")
    }
}

// ---------------------------------------------------------------------------

/// One line per service: status, name, load bar, and a few details.
pub fn service_list(shared: &Shared) -> impl View + use<> {
    let status = Status::new(0.65, 0.85, 1.0);
    let fill = Fill::horizontal([&themes::OK, &themes::OK, &themes::WARN, &themes::ERR]);
    let names: Vec<&str> = shared
        .lock()
        .unwrap()
        .services
        .iter()
        .map(|s| s.name)
        .collect();
    let mut layout = LinearLayout::vertical();

    for (i, name) in names.into_iter().enumerate() {
        // ● name          ████████▌       62%  ×4  region
        let load = {
            let shared = shared.clone();
            move || shared.lock().unwrap().services[i].load
        };
        let dot = {
            let (load, status) = (load.clone(), status.clone());
            TextView::live(move || {
                StyledString::styled("● ", ColorStyle::front(status.color(load())))
            })
        };
        let percent = {
            let load = load.clone();
            TextView::live(move || format!(" {:>3.0}%  ", load() * 100.0))
        };
        let details = {
            let shared = shared.clone();
            TextView::live(move || {
                let sim = shared.lock().unwrap();
                let service = &sim.services[i];
                StyledString::styled(
                    format!("×{:<2} {}", service.replicas, service.region),
                    ColorStyle::front(&themes::DIM),
                )
            })
        };
        let bar = Gauge::live(move || load().into())
            .style(BarStyle::Block)
            .fill(fill.clone())
            .track(&themes::SURFACE);

        layout.add_child(
            LinearLayout::horizontal()
                .child(dot)
                .child(TextView::new(name).fixed_width(14))
                .child(bar.full_width())
                .child(percent)
                .child(details.fixed_width(13)),
        );
    }
    layout
}

/// Network latency to each region: a dot and a name, and the latency on the right.
///
/// The label of each `Row` is a live `TextView`, since the dot changes color with the latency.
pub fn region_list(shared: &Shared) -> impl View + use<> {
    let status = Status::new(70.0, 130.0, 200.0);
    let names: Vec<&str> = shared
        .lock()
        .unwrap()
        .regions
        .iter()
        .map(|r| r.name)
        .collect();
    let mut layout = LinearLayout::vertical();

    for (i, name) in names.into_iter().enumerate() {
        let latency = {
            let shared = shared.clone();
            move || shared.lock().unwrap().regions[i].latency
        };
        let label = {
            let (latency, status) = (latency.clone(), status.clone());
            TextView::live(move || {
                let mut label =
                    StyledString::styled("● ", ColorStyle::front(status.color(latency())));
                label.append_plain(name);
                label
            })
            .no_wrap()
        };
        let status = status.clone();
        let value = TextView::live(move || {
            let latency = latency();
            StyledString::styled(
                format!("{latency:.0}ms"),
                ColorStyle::front(status.color(latency)),
            )
        })
        .h_align(HAlign::Right)
        .no_wrap();

        layout.add_child(Row::from_views(label, value));
    }
    layout
}

/// A few thin gauges: a name and a percentage, over a bar.
///
/// The gauges and the percentages read the simulation through closures.
pub fn health(shared: &Shared) -> impl View + use<> {
    let indicators: Vec<(&str, bool)> = shared
        .lock()
        .unwrap()
        .gauges
        .iter()
        .map(|g| (g.name, g.higher_is_better))
        .collect();
    let mut layout = LinearLayout::vertical();
    for (i, (name, higher_is_better)) in indicators.into_iter().enumerate() {
        // Red when low, then yellow, then the accent color; the other way round
        // for things where less is better.
        let steps = if higher_is_better {
            [
                (0.0, &themes::ERR),
                (0.3, &themes::WARN),
                (0.55, &themes::ACCENT),
            ]
        } else {
            [
                (0.0, &themes::ACCENT),
                (0.45, &themes::WARN),
                (0.7, &themes::ERR),
            ]
        };
        let fill = Fill::steps(steps);

        let value = {
            let shared = shared.clone();
            move || shared.lock().unwrap().gauges[i].value
        };
        let percent = {
            let (value, fill) = (value.clone(), fill.clone());
            TextView::live(move || {
                let value = value();
                let color = fill.color_for(value).expect("steps always give a color");
                StyledString::styled(format!("{:.0}%", value * 100.0), ColorStyle::front(color))
            })
            .h_align(HAlign::Right)
            .no_wrap()
        };

        if i > 0 {
            layout.add_child(DummyView);
        }
        layout.add_child(Row::new(name, percent));
        layout.add_child(
            Gauge::live(move || value().into())
                .fill(fill)
                .track(&themes::DIM),
        );
    }
    layout
}
