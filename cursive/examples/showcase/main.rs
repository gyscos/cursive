//! A fake "mission control" dashboard, showing what a full application can look like.
//!
//! It combines:
//! * A layout made of standard views (menubar, lists, dialogs, forms, scrollable logs).
//! * Standard data views (`Chart`, `Sparkline`, `Heatmap`) reading shared `Series`, colored
//!   with gradients from the theme.
//! * `Row`, `Gauge` and live `TextView` views for the health panel, the regions and the
//!   services, reading the simulation through closures.
//! * A single custom view, the title bar, and compositions of standard views for the rest (see
//!   `widgets.rs`).
//! * Shortcuts scoped to the dashboard with an `OnEventView`, so they stay quiet while a dialog
//!   is open.
//! * A background thread feeding live data to the interface.
//! * Themes that can be swapped while the application is running.
//!
//! Run it with `--demo` to watch it drive itself (`Cursive::play_events`).

mod sim;
mod themes;
mod widgets;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cursive::Cursive;
use cursive::align::HAlign;
use cursive::event::{Event, Key};
use cursive::menu;
use cursive::style::chart::Fill;
use cursive::style::gradient::Linear;
use cursive::style::{ColorStyle, Effect, PaletteColor, Style};
use cursive::traits::*;
use cursive::utils::markup::{StyledString, gradient};
use cursive::view::ScrollStrategy;
use cursive::views::{
    BarStyle, Chart, Checkbox, Dialog, DummyView, EditView, Heatmap, Layer, LinearLayout, ListView,
    OnEventView, PaddedView, Panel, Plot, ProgressBar, RadioGroup, Row, SelectView, SliderView,
    TabView, TextContent, TextView, ThemeOverlay, ValueRange,
};

use sim::{Level, LogLine, Metric, SERVICES, Shared, Sim};
use themes::SCHEMES;
use widgets::Header;

/// What callbacks need to get their job done.
///
/// It is stored as the cursive user data.
#[derive(Clone)]
struct App {
    /// The simulation, shared with the background thread and the views.
    shared: Shared,
    /// Content of the log views. Can be updated from any thread.
    log: TextContent,
    /// Index of the current color scheme.
    scheme: Arc<AtomicUsize>,
    /// Number of deployments started so far, to give each progress dialog its own name.
    deployments: Arc<AtomicUsize>,
}

impl App {
    fn get(siv: &mut Cursive) -> App {
        siv.user_data::<App>()
            .expect("user data was set in main")
            .clone()
    }

    /// Appends the last `n` log lines from the simulation to the log views.
    fn publish_logs(&self, sim: &Sim, n: usize) {
        for line in sim.logs.iter().skip(sim.logs.len().saturating_sub(n)) {
            let mut text = StyledString::plain("\n");
            text.append(render_log(line));
            self.log.append(text);
        }
    }

    /// Rebuilds the entire log content, forgetting about old lines.
    fn refresh_logs(&self, sim: &Sim) {
        let mut text = StyledString::new();
        for (i, line) in sim.logs.iter().enumerate() {
            if i > 0 {
                text.append_plain("\n");
            }
            text.append(render_log(line));
        }
        self.log.set_content(text);
    }

    /// Adds a new log line.
    fn log(&self, level: Level, source: &'static str, message: String) {
        let mut sim = self.shared.lock().unwrap();
        sim.log(level, source, message);
        self.publish_logs(&sim, 1);
    }
}

fn main() {
    let demo = std::env::args().any(|arg| arg == "--demo");

    let app = App {
        shared: Arc::new(Mutex::new(Sim::new())),
        log: TextContent::new(""),
        scheme: Arc::new(AtomicUsize::new(0)),
        deployments: Arc::new(AtomicUsize::new(0)),
    };
    app.refresh_logs(&app.shared.lock().unwrap());

    let mut siv = cursive::default();
    siv.set_window_title("cursive showcase");
    siv.set_user_data(app.clone());
    siv.set_theme(SCHEMES[0].theme());

    build_menubar(&mut siv);
    siv.add_fullscreen_layer(build_dashboard(&app));

    // These work whatever is on screen. The shortcuts of the dashboard itself
    // are in `build_dashboard`.
    siv.add_global_callback('q', Cursive::quit);
    // Global callbacks only see what the views ignored: a dialog that closes on Esc (see
    // `Dialog::dismiss_on`) keeps the event for itself, so this is Esc from anywhere else.
    siv.add_global_callback(Key::Esc, Cursive::select_menubar);
    siv.add_global_callback(Key::F10, Cursive::select_menubar);

    // The views read the simulation every time they are drawn, so all we
    // need is to redraw regularly.
    siv.set_fps(15);
    spawn_simulation(app);

    if demo {
        spawn_demo(&siv);
    }

    siv.run();
}

/// Runs the simulation in a background thread.
fn spawn_simulation(app: App) {
    thread::spawn(move || {
        let mut published = 0;
        loop {
            thread::sleep(Duration::from_millis(70));
            let mut sim = app.shared.lock().unwrap();
            if sim.paused {
                continue;
            }
            let new_lines = sim.step();
            published += new_lines;
            if published > 1000 {
                // Start again from a shorter backlog once in a while.
                published = 0;
                app.refresh_logs(&sim);
            } else {
                app.publish_logs(&sim, new_lines);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Layout

/// A titled box around a view: a standard `Panel`, with a left-aligned title and a column of
/// padding on each side. The rounded, dim border comes from the theme.
fn card<V: View>(title: &str, view: V) -> Panel<PaddedView<V>> {
    Panel::new(PaddedView::lrtb(1, 1, 0, 0, view))
        .title(title)
        .title_position(HAlign::Left)
        .title_delimiters(false)
}

fn build_dashboard(app: &App) -> impl View {
    let shared = &app.shared;

    // The pages, and the titles that pick one: the bar goes in the sidebar, the pages
    // next to it.
    let mut pages = TabView::new()
        .tab("◈  Overview", overview_page(app))
        .tab("▦  Cluster", cluster_page(app))
        .tab("≡  Logs", card("Logs", log_view(app, true)).full_screen());
    let nav = pages.detached_bar();

    let sidebar = LinearLayout::vertical()
        .child(card("Views", nav))
        .child(card("Regions", widgets::region_list(shared)))
        .child(card("Shortcuts", shortcuts()))
        .child(card("Health", widgets::health(shared)).full_height())
        .fixed_width(24);

    let root = LinearLayout::vertical()
        .child(Header::new(shared))
        .child(
            LinearLayout::horizontal()
                .child(sidebar)
                .child(pages.with_name("pages").full_screen()),
        )
        .full_screen();

    // The global theme draws views on a lighter "surface" color, which makes
    // dialogs and menus stand out. The dashboard itself sits directly on the
    // background color: here, and for everything below, that is the view color.
    let dashboard = ThemeOverlay::new(Layer::new(root), |theme| {
        theme.palette[PaletteColor::View] = theme.palette[PaletteColor::Background];
    });

    // Shortcuts that only apply when the dashboard is the top layer: not while
    // a dialog is open. Views inside get the event first, so a text field
    // still receives its own keys.
    let mut dashboard = OnEventView::new(dashboard)
        .on_event('d', deploy_dialog)
        .on_event('i', trigger_incident)
        .on_event('p', toggle_pause)
        .on_event('t', next_scheme);
    for page in 0..3 {
        let key = char::from(b'1' + page as u8);
        dashboard = dashboard.on_event(key, move |s| select_page(s, page));
    }
    dashboard
}

fn overview_page(app: &App) -> impl View {
    let shared = &app.shared;
    let tile = |title, metric| card(title, widgets::stat_tile(shared, metric)).full_width();

    LinearLayout::vertical()
        .child(
            LinearLayout::horizontal()
                .child(tile("CPU", Metric::Cpu))
                .child(tile("Memory", Metric::Memory))
                .child(tile("Throughput", Metric::Requests))
                .child(tile("Error rate", Metric::Errors))
                .fixed_height(6),
        )
        .child(
            LinearLayout::horizontal()
                .child(card("Requests / second", requests_chart(shared)).full_screen())
                .child(card("Latency (ms)", latency_chart(shared)).full_screen()),
        )
        .child(card("Live log", log_view(app, false)).fixed_height(9))
}

/// Requests per second, as a filled chart with a vertical gradient.
fn requests_chart(shared: &Shared) -> Chart {
    let requests = shared.lock().unwrap().requests.clone();
    Chart::new()
        .plot(Plot::area(requests).fill(Fill::vertical([&themes::ACCENT2, &themes::ACCENT])))
        // From zero to the next multiple of 1000 above the data, plus 10%.
        .range(ValueRange::auto().include(0.0).margin(0.1).step(1000.0))
}

/// Latency percentiles, as two lines.
fn latency_chart(shared: &Shared) -> Chart {
    let sim = shared.lock().unwrap();
    Chart::new()
        .plot(
            Plot::line(sim.p50.clone())
                .fill(&themes::ACCENT)
                .label("p50"),
        )
        .plot(
            Plot::line(sim.p99.clone())
                .fill(&themes::ACCENT2)
                .label("p99"),
        )
        .range(ValueRange::auto().include(0.0).margin(0.1).step(50.0))
        .legend(true)
}

/// Load of each node over time, two nodes per line.
fn node_heatmap(shared: &Shared) -> Heatmap {
    let sim = shared.lock().unwrap();
    let mut heatmap = Heatmap::new()
        .range(0.0..=1.0)
        .scale([
            &themes::SURFACE,
            &themes::ACCENT,
            &themes::WARN,
            &themes::ERR,
        ])
        .legend("idle", "saturated");
    for node in &sim.nodes {
        // "node-01" is shown as "01".
        heatmap.add_row(&node.name[5..], node.heat.clone());
    }
    heatmap
}

fn cluster_page(app: &App) -> impl View {
    let shared = &app.shared;

    LinearLayout::vertical()
        .child(card("Node load", node_heatmap(shared)).fixed_height(11))
        .child(card("Services", widgets::service_list(shared)).full_screen())
}

/// Creates a scrollable view on the shared log content.
fn log_view(app: &App, scrollbars: bool) -> impl View {
    TextView::new_with_content(app.log.clone())
        .no_wrap()
        .scrollable()
        .show_scrollbars(scrollbars)
        .scroll_strategy(ScrollStrategy::StickToBottom)
        .full_screen()
}

fn build_menubar(siv: &mut Cursive) {
    let mut themes = menu::Tree::new();
    for (i, scheme) in SCHEMES.iter().enumerate() {
        themes.add_leaf(scheme.name, move |s| set_scheme(s, i));
    }

    siv.menubar()
        .add_subtree(
            "Fleet",
            menu::Tree::new()
                .leaf("Deploy…", deploy_dialog)
                .leaf("Trigger incident", trigger_incident)
                .leaf("Pause / resume", toggle_pause)
                .delimiter()
                .leaf("Quit", Cursive::quit),
        )
        .add_subtree(
            "View",
            menu::Tree::new()
                .leaf("Overview", |s| select_page(s, 0))
                .leaf("Cluster", |s| select_page(s, 1))
                .leaf("Logs", |s| select_page(s, 2)),
        )
        .add_subtree("Theme", themes)
        .add_leaf("About", about_dialog);
    siv.set_autohide_menu(false);
}

/// Styles here refer to the palette rather than to fixed colors, so they
/// follow the theme without any extra work.
fn shortcuts() -> impl View {
    let key = Style::from(ColorStyle::front(PaletteColor::TitleSecondary)).combine(Effect::Bold);
    let mut layout = LinearLayout::vertical();
    for (keys, action) in [
        ("↑ ↓", "Switch view"),
        ("d", "Deploy…"),
        ("i", "Incident!"),
        ("t", "Next theme"),
        ("p", "Pause"),
        ("Esc", "Menu"),
        ("q", "Quit"),
    ] {
        layout.add_child(
            Row::new(
                StyledString::styled(keys, key),
                TextView::new(action).no_wrap(),
            )
            .label_width(3)
            .label_align(HAlign::Right)
            .gap(2),
        );
    }
    layout
}

/// The colors here are custom palette entries (see `themes.rs`): the text
/// follows the theme, even long after it was written.
fn render_log(line: &LogLine) -> StyledString {
    let (tag, color) = match line.level {
        Level::Info => ("INFO ", &themes::ACCENT),
        Level::Warn => ("WARN ", &themes::WARN),
        Level::Error => ("ERROR", &themes::ERR),
        Level::Deploy => ("SHIP ", &themes::ACCENT2),
    };
    let dim = ColorStyle::front(&themes::DIM);

    let mut text = StyledString::styled(format!("{} ", line.time), dim);
    text.append_styled(tag, Style::from(color).combine(Effect::Bold));
    text.append_styled(format!(" {:<12}", line.source), dim);
    match line.level {
        Level::Info => text.append_plain(&line.message),
        _ => text.append_styled(&line.message, color),
    }
    text
}

// ---------------------------------------------------------------------------
// Actions

/// Shows the given page. The titles in the sidebar follow.
fn select_page(siv: &mut Cursive, page: usize) {
    siv.call_on_name("pages", |pages: &mut TabView| pages.set_active(page));
}

fn set_scheme(siv: &mut Cursive, id: usize) {
    App::get(siv).scheme.store(id, Ordering::Relaxed);
    // That's it: everything on screen refers to the palette.
    siv.set_theme(SCHEMES[id].theme());
}

fn next_scheme(siv: &mut Cursive) {
    let current = App::get(siv).scheme.load(Ordering::Relaxed);
    set_scheme(siv, (current + 1) % SCHEMES.len());
}

fn toggle_pause(siv: &mut Cursive) {
    let app = App::get(siv);
    let mut sim = app.shared.lock().unwrap();
    sim.paused = !sim.paused;
}

fn trigger_incident(siv: &mut Cursive) {
    let app = App::get(siv);
    let mut sim = app.shared.lock().unwrap();
    sim.trigger_incident();
    app.publish_logs(&sim, 2);
}

fn about_dialog(siv: &mut Cursive) {
    let scheme = &SCHEMES[App::get(siv).scheme.load(Ordering::Relaxed)];

    let logo = [
        "                     _           ",
        "  ___ _   _ _ __ ___(_)_   _____ ",
        " / __| | | | '__/ __| \\ \\ / / _ \\",
        "| (__| |_| | |  \\__ \\ |\\ V /  __/",
        " \\___|\\__,_|_|  |___/_| \\_/ \\___|",
    ]
    .join("\n");
    let mut text = gradient::decorate_front(logo, Linear::simple(scheme.accent, scheme.accent2));
    text.append_plain(
        "\n\nEverything on this screen is drawn by cursive:\n\
         standard views for the layout, menus and forms,\n\
         standard charts and heatmaps for the data,\n\
         and only one custom view, for the title bar.",
    );

    siv.add_layer(
        Dialog::around(TextView::new(text))
            .title("About")
            .title_delimiters(false)
            .dismiss_button("Nice")
            .dismiss_on(Key::Esc),
    );
}

// ---------------------------------------------------------------------------
// Deployment

fn deploy_dialog(siv: &mut Cursive) {
    // Only one deployment at a time.
    if siv.find_name::<Dialog>("deploy").is_some() {
        return;
    }

    let mut strategy = RadioGroup::new();

    let form = ListView::new()
        .child(
            "Service",
            SelectView::new()
                .popup()
                .with_all_str(SERVICES.iter().map(|&(name, _)| name))
                .selected(3)
                .with_name("deploy_service"),
        )
        .child(
            "Version",
            EditView::new()
                .content("v2.14.0")
                .with_name("deploy_version")
                .fixed_width(18),
        )
        .child(
            "Strategy",
            LinearLayout::horizontal()
                .child(strategy.button("rolling", "Rolling"))
                .child(DummyView)
                .child(strategy.button("canary", "Canary"))
                .child(DummyView)
                .child(strategy.button("blue/green", "Blue/green")),
        )
        .child(
            "Replicas",
            LinearLayout::horizontal()
                .child(
                    SliderView::horizontal(12)
                        .value(3)
                        .on_change(|s, value| {
                            s.call_on_name("deploy_replicas", |text: &mut TextView| {
                                text.set_content(format!(" {}", value + 1));
                            });
                        })
                        .with_name("deploy_slider"),
                )
                .child(TextView::new(" 4").with_name("deploy_replicas")),
        )
        .delimiter()
        .child(
            "Run migrations",
            Checkbox::new().checked().with_name("deploy_migrations"),
        )
        .child("Notify #ops", Checkbox::new().with_name("deploy_notify"));

    siv.add_layer(
        Dialog::around(form)
            .title("Deploy a service")
            .title_delimiters(false)
            .padding_lrtb(2, 2, 1, 1)
            .dismiss_button("Cancel")
            .button("Deploy", move |s| {
                let service = s
                    .call_on_name("deploy_service", |v: &mut SelectView| v.selected_id())
                    .flatten()
                    .unwrap_or(0);
                let version = s
                    .call_on_name("deploy_version", |v: &mut EditView| v.get_content())
                    .unwrap();
                let replicas = s
                    .call_on_name("deploy_slider", |v: &mut SliderView| v.get_value() + 1)
                    .unwrap();
                let migrations = s
                    .call_on_name("deploy_migrations", |v: &mut Checkbox| v.is_checked())
                    .unwrap();

                s.pop_layer();
                run_deployment(
                    s,
                    Deployment {
                        service,
                        version: version.to_string(),
                        strategy: *strategy.selection(),
                        replicas,
                        migrations,
                    },
                );
            })
            .dismiss_on(Key::Esc)
            .with_name("deploy"),
    );
}

struct Deployment {
    /// Index in the list of services.
    service: usize,
    version: String,
    strategy: &'static str,
    replicas: usize,
    migrations: bool,
}

/// Shows a progress bar while a (pretend) deployment runs in the background.
fn run_deployment(siv: &mut Cursive, deployment: Deployment) {
    let app = App::get(siv);
    let cb_sink = siv.cb_sink().clone();
    let name = SERVICES[deployment.service].0;
    let title = format!("Deploying {name} {}", deployment.version);
    // Other dialogs can be opened while this one runs (or another deployment started):
    // the task closes exactly its own dialog, not whichever layer is on top by then.
    let id = app.deployments.fetch_add(1, Ordering::Relaxed);
    let dialog_name = format!("deploy_progress_{id}");
    let stage_name = format!("deploy_stage_{id}");

    let mut stages = vec!["Building image", "Pushing to registry"];
    if deployment.migrations {
        stages.push("Running migrations");
    }
    stages.extend(["Rolling out", "Waiting for health checks"]);
    const STEPS: usize = 30;

    let task_dialog = dialog_name.clone();
    let task_stage = stage_name.clone();
    let progress = ProgressBar::new()
        .style(BarStyle::Block)
        .track(&themes::DIM)
        .range(0, stages.len() * STEPS)
        .with_task(move |counter| {
            // This closure runs in its own thread.
            // It reports progress through the counter, and can send callbacks
            // to the main thread when it needs to update the interface.
            app.log(
                Level::Deploy,
                name,
                format!(
                    "starting {} deployment of {} ({} replicas)",
                    deployment.strategy, deployment.version, deployment.replicas
                ),
            );
            for stage in stages {
                let stage_name = task_stage.clone();
                // If this fails, the application is gone (the user quit): stop quietly.
                if cb_sink
                    .send(Box::new(move |s| {
                        s.call_on_name(&stage_name, |text: &mut TextView| {
                            text.set_content(format!("{stage}…"));
                        });
                    }))
                    .is_err()
                {
                    return;
                }
                for _ in 0..STEPS {
                    thread::sleep(Duration::from_millis(22));
                    counter.tick(1);
                }
                app.log(
                    Level::Deploy,
                    name,
                    format!("{}: done", stage.to_lowercase()),
                );
            }

            app.shared.lock().unwrap().services[deployment.service].replicas = deployment.replicas;
            app.log(
                Level::Deploy,
                name,
                format!(
                    "{} is live on {} replicas ✔",
                    deployment.version, deployment.replicas
                ),
            );
            // Nothing left to do if the application is gone.
            let _ = cb_sink.send(Box::new(move |s| {
                let screen = s.screen_mut();
                if let Some(position) = screen.find_layer_from_name(&task_dialog) {
                    screen.remove_layer(position);
                }
            }));
        });

    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(TextView::new("Preparing…").with_name(stage_name))
                .child(DummyView)
                .child(progress.fixed_width(46)),
        )
        .title(title)
        .title_delimiters(false)
        .padding_lrtb(2, 2, 1, 1)
        .with_name(dialog_name),
    );
}

// ---------------------------------------------------------------------------
// Demo mode

/// Plays a scripted sequence of key presses.
///
/// This is what was used to record the animation in the readme. The script
/// ends with the key that quits the application.
fn spawn_demo(siv: &Cursive) {
    use Event::{Char, Key as K};

    let script = [
        // Look at the overview for a bit, then visit the other pages.
        (3500, K(Key::Down)),
        (3500, K(Key::Down)),
        (2000, K(Key::Up)),
        (400, K(Key::Up)),
        // Deploy something: fill the form...
        (1500, Char('d')),
        (900, K(Key::Enter)),
        (400, K(Key::Up)),
        (400, K(Key::Enter)),
        (500, K(Key::Down)),
        (300, K(Key::Backspace)),
        (200, Char('1')),
        (500, K(Key::Down)),
        (400, K(Key::Right)),
        (300, K(Key::Enter)),
        (500, K(Key::Down)),
        (300, K(Key::Right)),
        (150, K(Key::Right)),
        (150, K(Key::Right)),
        (500, K(Key::Down)),
        (400, K(Key::Down)),
        (300, Char(' ')),
        (500, K(Key::Tab)),
        (300, K(Key::Right)),
        // ... and watch it go.
        (600, K(Key::Enter)),
        // Then things go wrong.
        (5000, Char('i')),
        (3500, Char('2')),
        (4000, Char('1')),
        // Tour of the themes, starting from the menubar.
        (2500, K(Key::Esc)),
        (400, K(Key::Right)),
        (300, K(Key::Right)),
        (400, K(Key::Enter)),
        (400, K(Key::Down)),
        (500, K(Key::Enter)),
        (2500, Char('t')),
        (2500, Char('t')),
        (2500, Char('t')),
        (3000, Char('q')),
    ];

    siv.play_events(script.map(|(ms, event)| (Duration::from_millis(ms), event)));
}
