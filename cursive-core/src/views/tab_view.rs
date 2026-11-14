use crate::{
    Cursive, Printer, Vec2,
    align::HAlign,
    direction::{Absolute, Direction, Orientation, Relative},
    event::{AnyCb, Event, EventResult, Key, MouseButton, MouseEvent},
    rect::Rect,
    style::{PaletteStyle, StyleKey, StyleType},
    utils::markup::StyledString,
    view::{
        CannotFocus, IntoBoxedView, Selector, View, ViewNotFound, combine_layout_key,
        fresh_layout_key, layout_key_seed, layout_pass::current_pass,
    },
    views::BoxedView,
};

use parking_lot::Mutex;
use std::sync::Arc;

/// Style of the title of the active tab.
///
/// It is used for the active title when the bar has the focus. If the theme does not define it,
/// this is the same as [`PaletteStyle::Highlight`].
///
/// It can be set in the palette with `Palette::set_custom_style(&TAB_ACTIVE, style)`
/// or `Palette::set_style("tabs.active", style)`.
pub static TAB_ACTIVE: StyleKey = StyleKey::from_palette("tabs.active", PaletteStyle::Highlight);

/// Style of the title of the active tab when the bar does not have the focus.
///
/// If the theme does not define it, this is the same as [`PaletteStyle::HighlightInactive`].
///
/// It can be set in the palette with `Palette::set_custom_style(&TAB_INACTIVE, style)`
/// or `Palette::set_style("tabs.inactive", style)`.
pub static TAB_INACTIVE: StyleKey =
    StyleKey::from_palette("tabs.inactive", PaletteStyle::HighlightInactive);

/// Where the bar of a [`TabView`] is drawn, relative to the active view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TabPlacement {
    /// A line of titles above the active view.
    #[default]
    Top,
    /// A line of titles below the active view.
    Bottom,
    /// A column of titles on the left of the active view.
    Left,
    /// A column of titles on the right of the active view.
    Right,
}

impl TabPlacement {
    /// The axis along which the titles are laid out.
    fn titles_axis(self) -> Orientation {
        match self {
            TabPlacement::Top | TabPlacement::Bottom => Orientation::Horizontal,
            TabPlacement::Left | TabPlacement::Right => Orientation::Vertical,
        }
    }

    /// The axis that goes from the bar to the active view.
    fn separation_axis(self) -> Orientation {
        match self.titles_axis() {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
        }
    }

    /// Whether the bar comes before the active view along the separation axis.
    fn bar_first(self) -> bool {
        matches!(self, TabPlacement::Top | TabPlacement::Left)
    }
}

impl std::str::FromStr for TabPlacement {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "top" => Ok(TabPlacement::Top),
            "bottom" => Ok(TabPlacement::Bottom),
            "left" => Ok(TabPlacement::Left),
            "right" => Ok(TabPlacement::Right),
            _ => Err(()),
        }
    }
}

/// One of the two parts of a `TabView` that can have the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Bar,
    Content,
}

/// A rectangle in the coordinates of the `TabView`.
#[derive(Clone, Copy, Debug)]
struct Area {
    offset: Vec2,
    size: Vec2,
}

impl Area {
    fn new(offset: impl Into<Vec2>, size: impl Into<Vec2>) -> Self {
        Area {
            offset: offset.into(),
            size: size.into(),
        }
    }

    fn is_empty(&self) -> bool {
        self.size.x == 0 || self.size.y == 0
    }

    fn contains(&self, point: Vec2) -> bool {
        point.x >= self.offset.x
            && point.y >= self.offset.y
            && point.x < self.offset.x + self.size.x
            && point.y < self.offset.y + self.size.y
    }
}

type ChangeCallback = dyn Fn(&mut Cursive, usize) + Send + Sync;

// A closure giving the title of a live tab.
type TitleSource = dyn Fn() -> StyledString + Send + Sync;

type TitleFormat = dyn Fn(&StyledString, TabState) -> StyledString + Send + Sync;

/// What a tab title is being drawn for.
///
/// This is given to the formatter set with [`TabView::title_format`].
///
/// ```rust
/// use cursive_core::views::TabState;
///
/// // A marker for the active tab, only while the bar has the focus.
/// let marker = |state: TabState| {
///     if state.active && state.focused {
///         "> "
///     } else {
///         "  "
///     }
/// };
/// # let _ = marker;
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabState {
    /// Index of the tab.
    pub index: usize,
    /// Whether this is the active tab.
    pub active: bool,
    /// Whether the bar drawing the title has the focus.
    ///
    /// This is the focus of the bar for every tab, not only for the active one.
    pub focused: bool,
}

/// What a [`TabView`] and its bars agree on.
///
/// The views of the tabs stay in the `TabView`. This is what the bars need, and what they can
/// change.
struct Shared {
    titles: Vec<StyledString>,
    // The closure of each live tab, if any. Always as long as `titles`.
    sources: Vec<Option<Arc<TitleSource>>>,
    // How many entries of `sources` are set: a cheap way to know there is nothing to evaluate.
    live_count: usize,
    // The layout pass the live titles were last evaluated in (unset outside of any pass).
    evaluated_in: Option<u64>,
    // Formats the titles, if not the default.
    format: Option<Arc<TitleFormat>>,
    // The formatted titles, kept while they are valid for `titles_key`. Only used with `format`.
    rendered: Option<Arc<Rendered>>,
    // Index of the active tab. Always 0 when there are no tabs.
    active: usize,
    // Changes whenever the titles change (or their number).
    titles_key: u64,
    // The callback of the `TabView`, called for the changes made by the user in any bar.
    on_change: Option<Arc<ChangeCallback>>,
    // Cleared when the `TabView` is dropped: the bars have nothing left to control.
    owner_alive: bool,
}

type SharedState = Arc<Mutex<Shared>>;

/// The titles as the formatter makes them, for every state they can be drawn in.
struct Rendered {
    // The `titles_key` these were made for.
    key: u64,
    // For each tab, indexed with `Rendered::slot`.
    items: Vec<[StyledString; 4]>,
    // For each tab, the width of the widest of its renderings: the width of its cell.
    widths: Vec<usize>,
}

impl Rendered {
    fn new(format: &TitleFormat, titles: &[StyledString], key: u64) -> Self {
        let items: Vec<[StyledString; 4]> = titles
            .iter()
            .enumerate()
            .map(|(index, title)| {
                let render = |active, focused| {
                    format(
                        title,
                        TabState {
                            index,
                            active,
                            focused,
                        },
                    )
                };
                [
                    render(false, false),
                    render(false, true),
                    render(true, false),
                    render(true, true),
                ]
            })
            .collect();
        let widths = items
            .iter()
            .map(|states| states.iter().map(|title| title.width()).max().unwrap_or(0))
            .collect();
        Rendered { key, items, widths }
    }

    fn slot(active: bool, focused: bool) -> usize {
        usize::from(active) * 2 + usize::from(focused)
    }
}

/// The titles of a bar, and how they are measured and drawn.
struct Titles<'a> {
    titles: &'a [StyledString],
    // Set when there is a formatter, and its output is up to date.
    rendered: Option<&'a Rendered>,
}

impl Titles<'_> {
    fn len(&self) -> usize {
        self.titles.len()
    }

    // Width of the cell of a title in a horizontal bar, or of the column for the widest one.
    //
    // By default this is the title and one space of padding on each side.
    fn cell(&self, index: usize) -> usize {
        match self.rendered {
            Some(rendered) => rendered.widths[index],
            None => self.titles[index].width() + 2,
        }
    }
}

/// Formats the titles if needed, and calls `f` with them and the active index.
///
/// The formatter runs before the state is locked, and only if the formatted titles are out of
/// date: that is when titles are measured. Without a formatter this only locks the state.
fn with_titles<R>(shared: &Mutex<Shared>, f: impl FnOnce(&Titles, usize) -> R) -> R {
    let rendered = rendered_titles(shared);
    let shared = shared.lock();
    // Titles that changed in between are measured by default, rather than wrongly.
    let rendered = rendered
        .as_deref()
        .filter(|rendered| rendered.key == shared.titles_key);
    f(
        &Titles {
            titles: &shared.titles,
            rendered,
        },
        shared.active,
    )
}

/// Gives the formatted titles, or `None` if there is no formatter.
fn rendered_titles(shared: &Mutex<Shared>) -> Option<Arc<Rendered>> {
    let (format, titles, key) = {
        let shared = shared.lock();
        let format = Arc::clone(shared.format.as_ref()?);
        if let Some(rendered) = &shared.rendered
            && rendered.key == shared.titles_key
        {
            return Some(Arc::clone(rendered));
        }
        (format, shared.titles.clone(), shared.titles_key)
    };

    // With the lock released: the formatter is not ours.
    let rendered = Arc::new(Rendered::new(&*format, &titles, key));

    let mut shared = shared.lock();
    if shared.titles_key == key {
        shared.rendered = Some(Arc::clone(&rendered));
    }
    Some(rendered)
}

/// Evaluates the live titles, unless they were already evaluated during this layout pass.
///
/// A title that changed is replaced, which changes `titles_key`. The closures run with the lock
/// released: they are taken out of the state first, and the results are stored afterwards.
///
/// Does nothing (and takes the lock only once) when no title is live.
fn refresh_live(shared: &Mutex<Shared>) {
    let pass = current_pass();
    let jobs: Vec<(usize, Arc<TitleSource>)> = {
        let mut shared = shared.lock();
        if shared.live_count == 0 {
            return;
        }
        if pass.is_some() && shared.evaluated_in == pass {
            return;
        }
        // Set before calling anything: a closure that comes back here does not loop.
        shared.evaluated_in = pass;
        shared
            .sources
            .iter()
            .enumerate()
            .filter_map(|(i, source)| source.as_ref().map(|source| (i, Arc::clone(source))))
            .collect()
    };

    let texts: Vec<StyledString> = jobs.iter().map(|(_, source)| source()).collect();

    let mut shared = shared.lock();
    let mut changed = false;
    for ((i, source), text) in jobs.into_iter().zip(texts) {
        // The tab may have changed while the lock was released.
        let current = shared
            .sources
            .get(i)
            .and_then(Option::as_ref)
            .is_some_and(|current| Arc::ptr_eq(current, &source));
        // Equal text is not a change: no bar needs to measure again.
        if current && shared.titles[i] != text {
            shared.titles[i] = text;
            changed = true;
        }
    }
    if changed {
        shared.titles_key = fresh_layout_key();
    }
}

/// Makes the given tab the active one, as asked by the user in a bar.
///
/// Gives the result to return: it runs the callback of the `TabView`, then `extra` (the callback
/// of the bar that was used, if any). The callbacks are not called if the tab is already active.
fn switch_to(
    shared: &Mutex<Shared>,
    index: usize,
    extra: Option<Arc<ChangeCallback>>,
) -> EventResult {
    let owner = {
        let mut shared = shared.lock();
        if index >= shared.titles.len() || index == shared.active {
            return EventResult::consumed();
        }
        shared.active = index;
        shared.on_change.clone()
    };
    if owner.is_none() && extra.is_none() {
        return EventResult::consumed();
    }
    EventResult::with_cb(move |siv| {
        if let Some(callback) = &owner {
            callback(siv, index);
        }
        if let Some(callback) = &extra {
            callback(siv, index);
        }
    })
}

/// What a bar does with a key.
enum BarKey {
    /// Select this tab.
    Switch(usize),
    /// A key for the bar, but it cannot do anything with it (the end of the bar).
    Ignored,
    /// Not a key of the bar.
    Other,
}

/// How a bar of titles looks and is used. This is shared by the bar of a `TabView` and `TabBar`.
#[derive(Clone, Copy)]
struct BarLayout {
    orientation: Orientation,
    align: HAlign,
}

impl BarLayout {
    fn horizontal(self) -> bool {
        self.orientation == Orientation::Horizontal
    }

    // Length of a title along the bar: its cell, padding included.
    fn title_length(self, titles: &Titles, index: usize) -> usize {
        if self.horizontal() {
            titles.cell(index)
        } else {
            1
        }
    }

    // Total length of all titles along the bar.
    fn titles_length(self, titles: &Titles) -> usize {
        (0..titles.len())
            .map(|i| self.title_length(titles, i))
            .sum()
    }

    // Size of the bar in the direction that is not along the titles.
    fn thickness(self, titles: &Titles) -> usize {
        if titles.len() == 0 {
            0
        } else if self.horizontal() {
            1
        } else {
            (0..titles.len()).map(|i| titles.cell(i)).max().unwrap_or(0)
        }
    }

    // Where each visible title goes in a bar `available` cells long.
    //
    // Gives `(tab index, offset, length)` along the bar. The last title may stick out.
    fn title_cells(
        self,
        titles: &Titles,
        active: usize,
        available: usize,
    ) -> Vec<(usize, usize, usize)> {
        if titles.len() == 0 {
            return Vec::new();
        }

        // The first title that keeps the active one in view.
        let mut start = active;
        let mut used = self.title_length(titles, start);
        while start > 0 {
            let previous = self.title_length(titles, start - 1);
            if used + previous > available {
                break;
            }
            used += previous;
            start -= 1;
        }

        let mut offset = if self.horizontal() {
            // This is 0 if the titles do not fit.
            self.align.get_offset(self.titles_length(titles), available)
        } else {
            0
        };

        let mut cells = Vec::new();
        for i in start..titles.len() {
            if offset >= available {
                break;
            }
            let length = self.title_length(titles, i);
            cells.push((i, offset, length));
            offset += length;
        }
        cells
    }

    // Finds the title at the given position of a bar of the given size.
    fn title_at(self, titles: &Titles, active: usize, size: Vec2, position: Vec2) -> Option<usize> {
        let (along, available) = if self.horizontal() {
            (position.x, size.x)
        } else {
            (position.y, size.y)
        };
        self.title_cells(titles, active, available)
            .into_iter()
            .find(|&(_, offset, length)| (offset..offset + length).contains(&along))
            .map(|(i, _, _)| i)
    }

    // Decides what a key does. Arrows only count along the bar, and never wrap around.
    fn key(self, event: &Event, active: usize, len: usize) -> BarKey {
        let Some(last) = len.checked_sub(1) else {
            return BarKey::Other;
        };
        let (previous, next) = if self.horizontal() {
            (Key::Left, Key::Right)
        } else {
            (Key::Up, Key::Down)
        };
        match *event {
            Event::Key(Key::Home) if active > 0 => BarKey::Switch(0),
            Event::Key(Key::End) if active < last => BarKey::Switch(last),
            Event::Key(Key::Home | Key::End) => BarKey::Ignored,
            Event::Key(key) if key == previous => match active.checked_sub(1) {
                Some(index) => BarKey::Switch(index),
                None => BarKey::Ignored,
            },
            Event::Key(key) if key == next && active < last => BarKey::Switch(active + 1),
            Event::Key(key) if key == next => BarKey::Ignored,
            _ => BarKey::Other,
        }
    }

    fn draw(self, printer: &Printer, titles: &Titles, active: usize) {
        let available = if self.horizontal() {
            printer.size.x
        } else {
            printer.size.y
        };
        for (i, offset, length) in self.title_cells(titles, active, available) {
            let (position, width) = if self.horizontal() {
                (Vec2::new(offset, 0), length)
            } else {
                // The highlight goes across the whole column.
                (Vec2::new(0, offset), printer.size.x)
            };
            let is_active = i == active;
            match titles.rendered {
                // The formatter decides the spacing.
                Some(rendered) => {
                    let slot = Rendered::slot(is_active, printer.focused);
                    draw_title(
                        printer,
                        position,
                        width,
                        &rendered.items[i][slot],
                        is_active,
                        0,
                    );
                }
                None => draw_title(printer, position, width, &titles.titles[i], is_active, 1),
            }
        }
    }
}

// Draws a title in `width` cells, after `padding` cells.
fn draw_title(
    printer: &Printer,
    position: Vec2,
    width: usize,
    title: &StyledString,
    active: bool,
    padding: usize,
) {
    // The style of the active tab goes over the style of the title.
    let highlight = active.then(|| {
        if printer.focused {
            StyleType::from(&TAB_ACTIVE)
        } else {
            StyleType::from(&TAB_INACTIVE)
        }
    });
    let with_highlight = |printer: &Printer, f: &dyn Fn(&Printer)| match highlight {
        Some(highlight) => printer.with_style(highlight, f),
        None => f(printer),
    };

    with_highlight(printer, &|printer| {
        printer.print_hline(position, width, " ");
    });

    let mut x = position.x + padding;
    for span in title.spans() {
        printer.with_style(*span.attr, |printer| {
            with_highlight(printer, &|printer| {
                printer.print((x, position.y), span.content);
            });
        });
        x += span.width;
    }
}

/// A view showing one of several views, picked with a bar of titles.
///
/// Each tab has a title and a view. A bar of titles lets the user choose which tab is active, and
/// only the active tab is sized, laid out and drawn. Other tabs keep their state.
///
/// # Layout
///
/// The bar is a line of titles above or below the active view, or a column of titles on its
/// left or right (see [`TabPlacement`]). Each title is shown with one space of padding on both
/// sides. In a column, the bar is as wide as the widest title, plus padding.
///
/// If the titles of a horizontal bar do not fit in the available width, the bar scrolls: it
/// starts at the first title that keeps the active one in view. The same goes for a column that
/// is not tall enough. Nothing is ever drawn outside of the bar.
///
/// The bar can be hidden with [`set_show_bar`](Self::set_show_bar), leaving only the active tab.
/// It can also be moved somewhere else in the layout, with
/// [`detached_bar`](Self::detached_bar).
///
/// # Focus
///
/// A `TabView` has two parts that can take the focus: the bar and the active view.
/// The bar can only be focused when it is shown and there is more than one tab: with a single
/// tab there is nothing to switch to.
///
/// * An arrow key that the focused part ignores moves the focus between the bar and the active
///   view, if the key goes along the axis that separates them. Other keys are left unhandled, so
///   a parent can move the focus elsewhere.
/// * `Tab` and `Shift+Tab` move the focus from one part to the next, like in a
///   [`LinearLayout`](crate::views::LinearLayout).
/// * With the bar focused, the arrow keys along the bar select the previous or next tab (without
///   wrapping around: at the ends the key is ignored), and `Home` and `End` select the first
///   and the last tab.
/// * A mouse press on a title selects it and focuses the bar. A mouse press on the active view
///   focuses it, if it can take the focus.
///
/// Changing the tab does not move the focus into the active view when the bar has it. If the
/// active view had the focus, the view of the new tab is offered it instead (see
/// [`set_active`](Self::set_active)).
///
/// # Style
///
/// The active title uses [`TAB_ACTIVE`] when the bar has the focus, and [`TAB_INACTIVE`]
/// when it does not. With a theme that defines neither, these are
/// [`PaletteStyle::Highlight`] and [`PaletteStyle::HighlightInactive`]. Other titles use the
/// current style.
///
/// Titles can be styled strings. The style of the active tab is applied over the style of the
/// title: where both set the same attribute (a color for example), the active style wins.
///
/// [`title_format`](Self::title_format) changes the text drawn for each title (brackets around
/// the active one, say), and [`tab_live`](Self::tab_live) makes a title follow a closure.
///
/// # A bar somewhere else
///
/// [`detached_bar`](Self::detached_bar) gives a [`TabBar`]: a separate view that shows the
/// titles of this `TabView`, and that can be put anywhere in the layout (in a sidebar, say).
/// The `TabView` then only draws the active tab.
///
/// ```rust
/// use cursive_core::views::{LinearLayout, Panel, TabView, TextView};
///
/// let mut pages = TabView::new()
///     .tab("Overview", TextView::new("Everything is fine."))
///     .tab("Logs", TextView::new("Nothing to report."));
/// let bar = pages.detached_bar();
///
/// let layout = LinearLayout::horizontal()
///     .child(Panel::new(bar))
///     .child(pages);
/// ```
///
/// There is no blueprint for a detached bar: it would need a way for two views to share their
/// state across a configuration. Build it in code, or use the `TabView` blueprint with its own
/// bar.
///
/// # Examples
///
/// ```rust
/// use cursive_core::Cursive;
/// use cursive_core::views::{TabPlacement, TabView, TextView};
///
/// let tabs = TabView::new()
///     .tab("Home", TextView::new("Welcome!"))
///     .tab("Settings", TextView::new("Nothing to configure."))
///     .bar_placement(TabPlacement::Left)
///     .on_change(|_siv: &mut Cursive, index: usize| {
///         // The user picked a tab.
///         let _ = index;
///     });
///
/// let mut siv = Cursive::new();
/// siv.add_layer(tabs);
/// ```
pub struct TabView {
    // The views of the tabs. The titles and the active index are in `shared`.
    tabs: Vec<BoxedView>,
    shared: SharedState,
    placement: TabPlacement,
    align: HAlign,
    show_bar: bool,

    // Whether this view holds the focus: a parent (or a mouse press, or `focus_view`) gave it,
    // and the parent did not take it back with `Event::FocusLost` since.
    held: bool,
    // The part that has the focus while it is held, and that gets it back when it is given again.
    // `None` when no part could take it (the active view refused). Kept across `FocusLost`.
    // Use `focus_part()`: the bar may not be focusable.
    focus: Option<Part>,

    // Size given by the last call to `layout`.
    last_size: Vec2,

    // The active index the `TabView` has dealt with. If the shared one is different, a bar
    // changed the active tab since.
    seen_active: usize,
    // The titles key and the active index used by the last call to `layout`.
    laid_out: (u64, usize),
    // The result of telling a view that it lost the focus, when it happened outside of an event.
    pending: EventResult,
    // Set when `pending` has already waited for an event that was consumed, and met one that was
    // not (see `deliver_pending`).
    pending_waited: bool,

    // Changes whenever something that affects the size changes (except what is in `shared`
    // and the active view).
    structure: u64,
    // Set when the layout must be done again.
    invalidated: bool,
}

new_default!(TabView);

impl Drop for TabView {
    fn drop(&mut self) {
        // The bars keep what they know, but there is nothing left to control.
        let mut shared = self.shared.lock();
        shared.owner_alive = false;
        shared.on_change = None;
        // The titles freeze: nothing is left to evaluate them for, and what the closures captured
        // is released. The formatter stays, so that the titles are still drawn the same way.
        shared.sources.iter_mut().for_each(|source| *source = None);
        shared.live_count = 0;
    }
}

impl TabView {
    /// Creates a new empty `TabView`.
    ///
    /// By default the bar is shown at the top, with the titles aligned to the left.
    pub fn new() -> Self {
        let titles_key = fresh_layout_key();
        TabView {
            tabs: Vec::new(),
            shared: Arc::new(Mutex::new(Shared {
                titles: Vec::new(),
                sources: Vec::new(),
                live_count: 0,
                evaluated_in: None,
                format: None,
                rendered: None,
                active: 0,
                titles_key,
                on_change: None,
                owner_alive: true,
            })),
            placement: TabPlacement::Top,
            align: HAlign::Left,
            show_bar: true,
            held: false,
            focus: Some(Part::Bar),
            last_size: Vec2::zero(),
            seen_active: 0,
            laid_out: (titles_key, 0),
            pending: EventResult::Ignored,
            pending_waited: false,
            structure: fresh_layout_key(),
            invalidated: true,
        }
    }

    /// Returns the number of tabs.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Returns `true` if there are no tabs.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Returns the index of the active tab, or `None` if there are no tabs.
    pub fn active(&self) -> Option<usize> {
        if self.tabs.is_empty() {
            None
        } else {
            Some(self.active_index())
        }
    }

    /// Makes the given tab the active one.
    ///
    /// Returns `Err(ViewNotFound)` and does nothing if `index >= self.len()`.
    ///
    /// This does not call the [`on_change`](Self::set_on_change) callback (it is only for changes
    /// made by the user).
    ///
    /// If the content of the previous tab had the focus (and this view still holds the focus),
    /// its view receives [`Event::FocusLost`], and the view of the new tab is offered the focus
    /// with [`take_focus`](View::take_focus). If it accepts it, it keeps the focus; if it
    /// refuses, the focus moves to the bar when the bar can take it, and otherwise nothing has
    /// the focus until the `TabView` is given the focus again (key events are ignored in the
    /// meantime). If the bar had the focus, the focus does not move. If this view does not hold
    /// the focus (its parent took it away), no view is notified: the new active view is offered
    /// the focus when the parent gives it back.
    ///
    /// # Callbacks
    ///
    /// There is no event to return the callbacks of the views (the ones they give back for
    /// [`Event::FocusLost`] and `take_focus`) to, so they are kept until the `TabView` handles
    /// an event. They are returned with the first event the `TabView` consumes. An
    /// [`EventResult`] cannot be "ignored, with a callback", and reporting an unused key as
    /// consumed would stop a parent from using it (to move the focus, say). So an event that
    /// is ignored lets the callbacks wait once; if the next event is ignored too, it is reported
    /// as consumed so that they run.
    pub fn set_active(&mut self, index: usize) -> Result<(), ViewNotFound> {
        if index >= self.tabs.len() {
            return Err(ViewNotFound);
        }
        self.sync();
        if index != self.active_index() {
            let had_focus = self.leave_content();
            self.shared.lock().active = index;
            self.seen_active = index;
            self.invalidate();
            if had_focus {
                self.offer_focus();
            }
        }
        Ok(())
    }

    /// Adds a tab at the end.
    ///
    /// If this is the first tab, it becomes the active one.
    pub fn add_tab<S, V>(&mut self, title: S, view: V)
    where
        S: Into<StyledString>,
        V: IntoBoxedView,
    {
        self.insert_tab(self.tabs.len(), title, view);
    }

    /// Adds a tab at the end.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn tab<S, V>(self, title: S, view: V) -> Self
    where
        S: Into<StyledString>,
        V: IntoBoxedView,
    {
        crate::With::with(self, |s| s.add_tab(title, view))
    }

    /// Inserts a tab at the given position.
    ///
    /// If `index` is larger than the number of tabs, the tab is added at the end.
    ///
    /// The active tab does not change, even if its index does. The exception is when there were
    /// no tabs: the new one becomes the active one, and if this view holds the focus it is offered
    /// the focus.
    pub fn insert_tab<S, V>(&mut self, index: usize, title: S, view: V)
    where
        S: Into<StyledString>,
        V: IntoBoxedView,
    {
        self.insert_entry(
            index,
            title.into(),
            None,
            BoxedView::new(view.into_boxed_view()),
        );
    }

    /// Adds a tab at the end, with a title that follows a closure.
    ///
    /// See [`tab_live`](Self::tab_live).
    pub fn add_tab_live<F, S, V>(&mut self, title: F, view: V)
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Into<StyledString>,
        V: IntoBoxedView,
    {
        self.insert_tab_live(self.tabs.len(), title, view);
    }

    /// Adds a tab at the end, with a title that follows a closure.
    ///
    /// Chainable variant of [`add_tab_live`](Self::add_tab_live).
    ///
    /// The title is the text the closure gives, re-evaluated as the application state changes.
    /// This is for titles like an unread counter, where the application would otherwise have to
    /// call [`set_title`](Self::set_title) from everywhere the count changes.
    ///
    /// # When the closure runs
    ///
    /// Like [`TextView::live`](crate::views::TextView::live), the closure runs at most once per
    /// layout pass, which is once per screen refresh. That is the first time during a refresh that
    /// the `TabView` or any of its bars (the built-in one or a [`TabBar`]) is asked whether it
    /// needs a relayout, for its layout key, for its size, or to lay out. All the live titles
    /// of a `TabView` are evaluated together, and the bars share the result: the text that is
    /// drawn is the text that was measured. If the text is equal to the current title, nothing
    /// changes. If it is different, the title is replaced as [`set_title`](Self::set_title)
    /// would, so every bar measures again.
    ///
    /// The closure never runs while drawing. It does not run when the tab is added either: the
    /// title is empty until the first layout. A `TabView` without a live title never calls any
    /// closure.
    ///
    /// Refreshes happen after events and callbacks, and every frame when
    /// [`Cursive::set_fps`](crate::Cursive::set_fps) is set: set it if the state can change
    /// without any event.
    ///
    /// Calling the methods of the view directly, outside of a refresh (for example in a test), is
    /// not part of a layout pass: every such call may run the closure again.
    ///
    /// The closure runs on the UI thread, in the middle of the layout, but not while the state
    /// shared with the bars is locked:
    ///
    /// * It must be fast.
    /// * It must not call back into [`Cursive`].
    ///
    /// # Changing the title by other means
    ///
    /// [`set_title`](Self::set_title) stops using the closure of that tab: the title it was given
    /// stays. Use [`set_title_source`](Self::set_title_source) to make a tab live again.
    ///
    /// A live title can't be given in a config file (there is no blueprint for closures).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::views::{TabView, TextView};
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// let unread = Arc::new(AtomicUsize::new(0));
    ///
    /// let tabs = TabView::new()
    ///     .tab("Overview", TextView::new("Everything is fine."))
    ///     .tab_live(
    ///         {
    ///             let unread = Arc::clone(&unread);
    ///             move || format!("Logs ({})", unread.load(Ordering::Relaxed))
    ///         },
    ///         TextView::new("Nothing to report."),
    ///     );
    ///
    /// // Later, from anywhere: the next refresh shows the new value.
    /// unread.fetch_add(1, Ordering::Relaxed);
    /// ```
    #[must_use]
    pub fn tab_live<F, S, V>(self, title: F, view: V) -> Self
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Into<StyledString>,
        V: IntoBoxedView,
    {
        crate::With::with(self, |s| s.add_tab_live(title, view))
    }

    /// Inserts a tab at the given position, with a title that follows a closure.
    ///
    /// The position and the active tab are handled like in [`insert_tab`](Self::insert_tab).
    /// See [`tab_live`](Self::tab_live) for when the closure runs.
    pub fn insert_tab_live<F, S, V>(&mut self, index: usize, title: F, view: V)
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Into<StyledString>,
        V: IntoBoxedView,
    {
        self.insert_entry(
            index,
            StyledString::new(),
            Some(Self::source(title)),
            BoxedView::new(view.into_boxed_view()),
        );
    }

    fn source<F, S>(title: F) -> Arc<TitleSource>
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Into<StyledString>,
    {
        Arc::new(move || title().into())
    }

    fn insert_entry(
        &mut self,
        index: usize,
        title: StyledString,
        source: Option<Arc<TitleSource>>,
        view: BoxedView,
    ) {
        self.sync();
        let was_empty = self.tabs.is_empty();
        let index = index.min(self.tabs.len());
        {
            let mut shared = self.shared.lock();
            if !shared.titles.is_empty() && index <= shared.active {
                shared.active += 1;
            }
            shared.titles.insert(index, title);
            if source.is_some() {
                shared.live_count += 1;
                shared.evaluated_in = None;
            }
            shared.sources.insert(index, source);
            shared.titles_key = fresh_layout_key();
            self.seen_active = shared.active;
        }
        self.tabs.insert(index, view);
        self.invalidate();
        if was_empty && self.held {
            self.offer_focus();
        }
    }

    /// Removes a tab, and returns its view.
    ///
    /// Returns `None` and does nothing if `index >= self.len()`.
    ///
    /// The active tab does not change, unless it is the one being removed: then the next tab
    /// becomes the active one, or the previous one if it was the last. Neither the focus nor
    /// the [`on_change`](Self::set_on_change) callback are affected.
    ///
    /// If the removed tab was the active one and its content had the focus, the new active view
    /// is offered the focus, like in [`set_active`](Self::set_active). The removed view is
    /// handed back as it is, like [`LinearLayout::remove_child`] and
    /// [`StackView::pop_layer`] do: it does not receive [`Event::FocusLost`]. If the bar had the
    /// focus and there is only one tab left, the bar cannot be focused anymore: the active view
    /// is offered the focus instead.
    ///
    /// If the last tab is removed, no part can have the focus. The `TabView` still holds the
    /// focus its parent gave it though: a tab added later is offered the focus (see
    /// [`insert_tab`](Self::insert_tab)).
    ///
    /// The callbacks the views give back when the focus moves are kept until the next event, like
    /// for [`set_active`](Self::set_active).
    ///
    /// [`LinearLayout::remove_child`]: crate::views::LinearLayout::remove_child
    /// [`StackView::pop_layer`]: crate::views::StackView::pop_layer
    pub fn remove_tab(&mut self, index: usize) -> Option<BoxedView> {
        if index >= self.tabs.len() {
            return None;
        }
        self.sync();
        let had_focus = index == self.active_index() && self.content_focused();
        let view = self.tabs.remove(index);
        {
            let mut shared = self.shared.lock();
            shared.titles.remove(index);
            if shared.sources.remove(index).is_some() {
                shared.live_count -= 1;
            }
            shared.titles_key = fresh_layout_key();
            if index < shared.active {
                shared.active -= 1;
            }
            // Covers removing the last tab (the previous one is selected),
            // and emptying the view.
            shared.active = shared.active.min(self.tabs.len().saturating_sub(1));
            self.seen_active = shared.active;
        }
        self.invalidate();
        if had_focus || self.bar_lost_focusable() {
            self.offer_focus();
        }
        Some(view)
    }

    /// Changes the title of a tab.
    ///
    /// Does nothing if `index >= self.len()`.
    ///
    /// If the tab is live (see [`tab_live`](Self::tab_live)), this stops using its closure: the
    /// title given here stays, like [`TextView::set_content`](crate::views::TextView::set_content)
    /// does for a live `TextView`. Use [`set_title_source`](Self::set_title_source) to make it
    /// live again.
    pub fn set_title<S>(&mut self, index: usize, title: S)
    where
        S: Into<StyledString>,
    {
        let title = title.into();
        {
            let mut shared = self.shared.lock();
            let Some(slot) = shared.titles.get_mut(index) else {
                return;
            };
            *slot = title;
            if shared.sources[index].take().is_some() {
                shared.live_count -= 1;
            }
            shared.titles_key = fresh_layout_key();
        }
        self.invalidate();
    }

    /// Sets how the titles are shown.
    ///
    /// The formatter gets the title and a [`TabState`], and gives the text to draw. Use it for an
    /// indicator of the active tab other than the highlight (brackets around the title, say), or
    /// one that depends on the focus of the bar. A live title (see
    /// [`tab_live`](Self::tab_live)) is formatted like any other.
    ///
    /// The built-in bar and every [`TabBar`] use it.
    ///
    /// # Spacing
    ///
    /// By default, a title is drawn with one space of padding on each side. With a formatter,
    /// its output is drawn as is: no padding is added around it. Add the spaces you want, like
    /// `" title "` or `"[title]"`.
    ///
    /// # Stable layout
    ///
    /// The cell of a title is as wide as the widest of the renderings it can have: inactive,
    /// active without the focus, and active with the focus (the focus of the bar is also given
    /// to the titles that are not active). This way the bar does not move when the active tab
    /// or the focus changes. A rendering narrower than the cell is drawn at its start, and the
    /// rest of the cell has the style of the title. A click anywhere in the cell selects the tab.
    ///
    /// The formatter runs when the titles are measured, with the state shared with the bars
    /// unlocked. The results are kept until the titles or the formatter change. It must be
    /// fast, and must not call back into [`Cursive`]. It may be called several times for each
    /// title.
    ///
    /// # Style
    ///
    /// [`TAB_ACTIVE`] and [`TAB_INACTIVE`] are applied on top of the formatted title, like they
    /// are for the default one. To mark the active tab without the highlight, give both keys a
    /// neutral style in the theme:
    ///
    /// ```rust
    /// use cursive_core::style::Style;
    /// use cursive_core::theme::Theme;
    /// use cursive_core::views::{TAB_ACTIVE, TAB_INACTIVE};
    ///
    /// let mut theme = Theme::retro();
    /// theme.palette.set_custom_style(&TAB_ACTIVE, Style::none());
    /// theme.palette.set_custom_style(&TAB_INACTIVE, Style::none());
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::views::{TabView, TextView};
    ///
    /// let tabs = TabView::new()
    ///     .tab("Overview", TextView::new("Everything is fine."))
    ///     .tab("Logs", TextView::new("Nothing to report."))
    ///     .title_format(|title, state| {
    ///         if state.active {
    ///             format!("[{}]", title.source()).into()
    ///         } else {
    ///             format!(" {} ", title.source()).into()
    ///         }
    ///     });
    /// ```
    pub fn set_title_format<F>(&mut self, format: F)
    where
        F: Fn(&StyledString, TabState) -> StyledString + Send + Sync + 'static,
    {
        {
            let mut shared = self.shared.lock();
            shared.format = Some(Arc::new(format));
            shared.rendered = None;
            shared.titles_key = fresh_layout_key();
        }
        self.invalidate();
    }

    /// Sets how the titles are shown.
    ///
    /// Chainable variant of [`set_title_format`](Self::set_title_format).
    #[must_use]
    pub fn title_format<F>(self, format: F) -> Self
    where
        F: Fn(&StyledString, TabState) -> StyledString + Send + Sync + 'static,
    {
        crate::With::with(self, |s| s.set_title_format(format))
    }

    /// Goes back to the default look of the titles.
    ///
    /// That is one space of padding on each side. Does nothing if no formatter was set with
    /// [`set_title_format`](Self::set_title_format).
    pub fn clear_title_format(&mut self) {
        {
            let mut shared = self.shared.lock();
            if shared.format.take().is_none() {
                return;
            }
            shared.rendered = None;
            shared.titles_key = fresh_layout_key();
        }
        self.invalidate();
    }

    /// Makes the title of a tab follow a closure.
    ///
    /// Does nothing if `index >= self.len()`. Replaces the closure of the tab, if it had one. The
    /// current title stays until the closure first runs, at the next layout. See
    /// [`tab_live`](Self::tab_live) for when the closure runs and what it must not do, and
    /// [`set_title`](Self::set_title) for how to stop using it.
    pub fn set_title_source<F, S>(&mut self, index: usize, title: F)
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Into<StyledString>,
    {
        let mut shared = self.shared.lock();
        let Some(slot) = shared.sources.get_mut(index) else {
            return;
        };
        if slot.replace(Self::source(title)).is_none() {
            shared.live_count += 1;
        }
        // Even within the current pass: this closure never ran.
        shared.evaluated_in = None;
    }

    /// Returns the view of the given tab.
    ///
    /// Returns `None` if `index >= self.len()`.
    pub fn get_tab(&self, index: usize) -> Option<&BoxedView> {
        self.tabs.get(index)
    }

    /// Returns the view of the given tab, mutably.
    ///
    /// Returns `None` if `index >= self.len()`.
    pub fn get_tab_mut(&mut self, index: usize) -> Option<&mut BoxedView> {
        self.tabs.get_mut(index)
    }

    /// Sets where the bar is drawn.
    ///
    /// The default is [`TabPlacement::Top`].
    pub fn set_bar_placement(&mut self, placement: TabPlacement) {
        if self.placement != placement {
            self.placement = placement;
            self.invalidate();
        }
    }

    /// Sets where the bar is drawn.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn bar_placement(self, placement: TabPlacement) -> Self {
        crate::With::with(self, |s| s.set_bar_placement(placement))
    }

    /// Sets how the titles are aligned in a bar at the top or at the bottom.
    ///
    /// This only matters when the titles do not use the whole width. Titles in a column are not
    /// affected. The default is [`HAlign::Left`].
    pub fn set_bar_align(&mut self, align: HAlign) {
        self.align = align;
    }

    /// Sets how the titles are aligned in a bar at the top or at the bottom.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn bar_align(self, align: HAlign) -> Self {
        crate::With::with(self, |s| s.set_bar_align(align))
    }

    /// Sets whether the bar is shown. The default is `true`.
    ///
    /// Without the bar only the active tab is visible, and the user cannot change it (unless
    /// there is a [`TabBar`], see [`detached_bar`](Self::detached_bar)).
    ///
    /// This is only about the bar of the `TabView` itself: calling this with `true` after
    /// `detached_bar` shows the bar here too, in addition to the detached ones.
    pub fn set_show_bar(&mut self, show_bar: bool) {
        if self.show_bar != show_bar {
            self.show_bar = show_bar;
            self.invalidate();
            if self.bar_lost_focusable() {
                self.offer_focus();
            }
        }
    }

    /// Sets whether the bar is shown. The default is `true`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn show_bar(self, show_bar: bool) -> Self {
        crate::With::with(self, |s| s.set_show_bar(show_bar))
    }

    /// Returns a [`TabBar`]: a view showing the titles of this `TabView`, to place anywhere.
    ///
    /// The receiver stays the content: it keeps the views of the tabs, and only draws the active
    /// one. This hides the bar the `TabView` would draw itself (it calls
    /// [`set_show_bar(false)`](Self::set_show_bar); calling `set_show_bar(true)` afterwards shows
    /// both bars).
    ///
    /// The bar is vertical (one title per line, for a sidebar). Use [`TabBar::orientation`] for
    /// a horizontal one.
    ///
    /// This can be called several times: every bar shows and controls the same tabs, so they
    /// always agree. Everything that changes the tabs through the `TabView` ([`set_active`],
    /// [`add_tab`], [`insert_tab`], [`remove_tab`], [`set_title`]) is reflected by the bars, and
    /// the `TabView` follows what the user does in the bars, with no call needed from the
    /// application.
    ///
    /// # Focus
    ///
    /// When the user changes the tab from a bar while the active view of the `TabView` had the
    /// focus and the `TabView` still holds it, that view gets [`Event::FocusLost`] the next time
    /// the `TabView` is laid out or receives an event, and the view of the new tab is offered the
    /// focus instead, like in [`set_active`](Self::set_active). If the `TabView` had already lost
    /// the focus (the focus moved to the bar, say), the views are not told again: the view of
    /// the new tab is offered the focus when the `TabView` gets it back.
    ///
    /// # Callbacks
    ///
    /// When the user changes the tab from a bar, the [`on_change`](Self::set_on_change) callback
    /// of the `TabView` is called, then the one of the bar. Neither is called by changes made in
    /// code.
    ///
    /// # Lifetime
    ///
    /// A `TabBar` whose `TabView` was dropped still draws the titles it last knew, but cannot be
    /// used to change the tab or take the focus. A `TabView` whose bars were dropped behaves as
    /// if its bar was hidden.
    ///
    /// There is no blueprint for this: it would need two views to share their state across a
    /// configuration.
    ///
    /// [`set_active`]: Self::set_active
    /// [`add_tab`]: Self::add_tab
    /// [`insert_tab`]: Self::insert_tab
    /// [`remove_tab`]: Self::remove_tab
    /// [`set_title`]: Self::set_title
    pub fn detached_bar(&mut self) -> TabBar {
        self.set_show_bar(false);
        TabBar::with_shared(Arc::clone(&self.shared))
    }

    /// Sets a callback to run when the user changes the active tab.
    ///
    /// It receives the index of the new active tab.
    ///
    /// It is not called for changes made by code, like [`set_active`](Self::set_active). It is
    /// called for changes made in the bar of the `TabView`, and in a [`TabBar`] from
    /// [`detached_bar`](Self::detached_bar) (before the callback of that bar).
    pub fn set_on_change<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, usize) + 'static + Send + Sync,
    {
        self.shared.lock().on_change = Some(Arc::new(callback));
    }

    /// Sets a callback to run when the user changes the active tab.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_change<F>(self, callback: F) -> Self
    where
        F: Fn(&mut Cursive, usize) + 'static + Send + Sync,
    {
        crate::With::with(self, |s| s.set_on_change(callback))
    }

    // Marks the size as potentially changed.
    fn invalidate(&mut self) {
        self.structure = fresh_layout_key();
        self.invalidated = true;
    }

    fn active_index(&self) -> usize {
        self.shared.lock().active
    }

    // What the last layout depended on: the titles, and the active tab.
    fn shared_key(&self) -> (u64, usize) {
        let shared = self.shared.lock();
        (shared.titles_key, shared.active)
    }

    fn bar_style(&self) -> BarLayout {
        BarLayout {
            orientation: self.placement.titles_axis(),
            align: self.align,
        }
    }

    // Deals with a change of the active tab made by a bar since last time.
    //
    // If the content has the focus, it keeps it: the view that was active is told it lost the
    // focus, and the new one is offered it (see `offer_focus`). If this view does not hold the
    // focus, nobody is told: the new view is offered the focus when it is given.
    fn sync(&mut self) {
        let active = self.active_index();
        if active == self.seen_active {
            return;
        }
        let previous = std::mem::replace(&mut self.seen_active, active);
        if !self.content_focused() {
            return;
        }
        if let Some(view) = self.tabs.get_mut(previous) {
            let result = view.on_event(Event::FocusLost);
            self.keep_pending(result);
        }
        self.offer_focus();
    }

    // Keeps the callbacks of a result until there is an event to return them with.
    fn keep_pending(&mut self, result: EventResult) {
        if let EventResult::Consumed(Some(_)) = result {
            self.pending = std::mem::replace(&mut self.pending, EventResult::Ignored).and(result);
        }
    }

    // Combines the result of an event with the callbacks that are waiting.
    //
    // They go with an event that was consumed. An ignored event cannot carry them without being
    // reported as consumed, which would stop the parent from using it: it lets them wait once.
    fn deliver_pending(&mut self, result: EventResult) -> EventResult {
        if !matches!(self.pending, EventResult::Consumed(Some(_))) {
            return result;
        }
        if !result.is_consumed() && !self.pending_waited {
            self.pending_waited = true;
            return result;
        }
        self.pending_waited = false;
        std::mem::replace(&mut self.pending, EventResult::Ignored).and(result)
    }

    // For a change of the active tab made in code, before it happens.
    //
    // If the content has the focus, tells the active view that it is lost, and returns `true`.
    fn leave_content(&mut self) -> bool {
        if !self.content_focused() {
            return false;
        }
        let result = self.lost_content();
        self.keep_pending(result);
        true
    }

    // Tells the active view that it lost the focus, if it had it.
    fn lost_content(&mut self) -> EventResult {
        if !self.content_focused() {
            return EventResult::Ignored;
        }
        let active = self.active_index();
        match self.tabs.get_mut(active) {
            Some(view) => view.on_event(Event::FocusLost),
            None => EventResult::Ignored,
        }
    }

    // Gives the focus to the active view, or else to the bar, or else to nothing.
    //
    // For when the part that had the focus is gone: the active view changed, or the bar can't be
    // focused anymore.
    fn offer_focus(&mut self) {
        let active = self.active_index();
        let taken = self
            .tabs
            .get_mut(active)
            .and_then(|view| view.take_focus(Direction::none()).ok());
        match taken {
            Some(result) => {
                self.focus = Some(Part::Content);
                self.keep_pending(result);
            }
            None if self.bar_focusable() => self.focus = Some(Part::Bar),
            None => self.focus = None,
        }
    }

    // Whether this view holds the focus on a bar that can't take it anymore.
    fn bar_lost_focusable(&self) -> bool {
        self.held && self.focus == Some(Part::Bar) && !self.bar_focusable()
    }

    // Whether the active view has the focus: this view holds it, and it is the focused part.
    fn content_focused(&self) -> bool {
        self.held && self.focus_part() == Some(Part::Content)
    }

    // Records that a part was given the focus.
    fn give_focus(&mut self, part: Part) {
        self.focus = Some(part);
        self.held = true;
    }

    // Size of the bar in the direction that is not along the titles.
    fn bar_thickness(&self) -> usize {
        if !self.show_bar {
            return 0;
        }
        with_titles(&self.shared, |titles, _| self.bar_style().thickness(titles))
    }

    // Splits the given size between the bar and the active view.
    fn areas(&self, size: Vec2) -> (Area, Area) {
        match self.placement {
            TabPlacement::Top => {
                let t = self.bar_thickness().min(size.y);
                (
                    Area::new((0, 0), (size.x, t)),
                    Area::new((0, t), (size.x, size.y - t)),
                )
            }
            TabPlacement::Bottom => {
                let t = self.bar_thickness().min(size.y);
                (
                    Area::new((0, size.y - t), (size.x, t)),
                    Area::new((0, 0), (size.x, size.y - t)),
                )
            }
            TabPlacement::Left => {
                let t = self.bar_thickness().min(size.x);
                (
                    Area::new((0, 0), (t, size.y)),
                    Area::new((t, 0), (size.x - t, size.y)),
                )
            }
            TabPlacement::Right => {
                let t = self.bar_thickness().min(size.x);
                (
                    Area::new((size.x - t, 0), (t, size.y)),
                    Area::new((0, 0), (size.x - t, size.y)),
                )
            }
        }
    }

    // Finds the title at the given position of the bar.
    fn title_at(&self, bar: Area, position: Vec2) -> Option<usize> {
        with_titles(&self.shared, |titles, active| {
            self.bar_style()
                .title_at(titles, active, bar.size, position)
        })
    }

    fn bar_focusable(&self) -> bool {
        self.show_bar && self.tabs.len() > 1
    }

    // The part that has the focus (if the view has it), or that gets it when it is given.
    //
    // `None` if no part could take it.
    fn focus_part(&self) -> Option<Part> {
        match self.focus {
            Some(Part::Bar) if !self.bar_focusable() => Some(Part::Content),
            focus => focus,
        }
    }

    // The parts in the order they are on screen, along the separation axis.
    fn parts(&self) -> [Part; 2] {
        if self.placement.bar_first() {
            [Part::Bar, Part::Content]
        } else {
            [Part::Content, Part::Bar]
        }
    }

    // Moves the focus to the next (or previous) part, if it accepts it.
    //
    // `source` is where the focus comes from, as seen by the part receiving it.
    fn move_focus(&mut self, forward: bool, source: Direction) -> EventResult {
        let parts = self.parts();
        let Some(current) = self.focus_part() else {
            return EventResult::Ignored;
        };
        let position = parts.iter().position(|&part| part == current).unwrap();
        let target = if forward {
            position + 1
        } else {
            position.wrapping_sub(1)
        };
        let Some(&target) = parts.get(target) else {
            return EventResult::Ignored;
        };

        let active = self.active_index();
        let result = match target {
            Part::Bar if self.bar_focusable() => Some(EventResult::consumed()),
            Part::Bar => None,
            Part::Content => self
                .tabs
                .get_mut(active)
                .and_then(|view| view.take_focus(source).ok()),
        };
        let Some(result) = result else {
            return EventResult::Ignored;
        };

        let lost = self.lost_content();
        self.give_focus(target);
        lost.and(result)
    }

    // Moves the focus between parts, from an arrow key that the focused part did not use.
    fn arrow_focus(&mut self, key: Absolute) -> EventResult {
        let (orientation, relative) = key.split();
        if orientation != self.placement.separation_axis() {
            return EventResult::Ignored;
        }
        // Going down means the focus comes from above.
        self.move_focus(relative == Relative::Back, Direction::Abs(key.opposite()))
    }

    // Handles a tab switch requested by the user in the bar of this view.
    fn user_switch(&mut self, index: usize) -> EventResult {
        let changed = index != self.active_index() && index < self.tabs.len();
        let result = switch_to(&self.shared, index, None);
        if changed {
            self.seen_active = index;
            self.invalidate();
        }
        result
    }

    fn bar_event(&mut self, event: &Event) -> EventResult {
        let (active, len) = (self.active_index(), self.tabs.len());
        match self.bar_style().key(event, active, len) {
            BarKey::Switch(index) => self.user_switch(index),
            BarKey::Ignored => EventResult::Ignored,
            BarKey::Other => match *event {
                Event::Key(Key::Left) => self.arrow_focus(Absolute::Left),
                Event::Key(Key::Right) => self.arrow_focus(Absolute::Right),
                Event::Key(Key::Up) => self.arrow_focus(Absolute::Up),
                Event::Key(Key::Down) => self.arrow_focus(Absolute::Down),
                Event::Key(Key::Tab) => self.move_focus(true, Direction::front()),
                Event::Shift(Key::Tab) => self.move_focus(false, Direction::back()),
                _ => EventResult::Ignored,
            },
        }
    }

    // Handles an event the active view did not use.
    fn unused_content_event(&mut self, event: &Event) -> EventResult {
        match *event {
            Event::Key(Key::Tab) => self.move_focus(true, Direction::front()),
            Event::Shift(Key::Tab) => self.move_focus(false, Direction::back()),
            Event::Key(Key::Left) => self.arrow_focus(Absolute::Left),
            Event::Key(Key::Right) => self.arrow_focus(Absolute::Right),
            Event::Key(Key::Up) => self.arrow_focus(Absolute::Up),
            Event::Key(Key::Down) => self.arrow_focus(Absolute::Down),
            _ => EventResult::Ignored,
        }
    }

    fn mouse_event(&mut self, event: &Event) -> EventResult {
        let Event::Mouse {
            offset,
            position,
            event: mouse,
        } = *event
        else {
            return EventResult::Ignored;
        };
        let (bar, content) = self.areas(self.last_size);

        if !mouse.grabs_focus() {
            // Releases and drags go to the view that has the focus, wherever they are: a drag
            // that started in it can end before our top left corner (like `LinearLayout` does).
            if self.content_focused() {
                return self.forward(event, content);
            }
            return EventResult::Ignored;
        }

        let Some(position) = position.checked_sub(offset) else {
            return EventResult::Ignored;
        };

        if !bar.is_empty() && bar.contains(position) {
            let MouseEvent::Press(MouseButton::Left) = mouse else {
                return EventResult::Ignored;
            };
            // Tell the view that has the focus now, before the active tab changes.
            let lost = if self.bar_focusable() {
                let lost = self.lost_content();
                self.give_focus(Part::Bar);
                lost
            } else {
                EventResult::Ignored
            };
            let result = match self.title_at(bar, position - bar.offset) {
                Some(index) => self.user_switch(index),
                None => EventResult::consumed(),
            };
            return lost.and(result);
        }

        if content.contains(position) {
            let mut result = EventResult::Ignored;
            let active = self.active_index();
            if !self.content_focused()
                && let Some(view) = self.tabs.get_mut(active)
                && let Ok(taken) = view.take_focus(Direction::none())
            {
                self.give_focus(Part::Content);
                result = taken;
            }
            return result.and(self.forward(event, content));
        }

        EventResult::Ignored
    }

    // Sends an event to the active view, which is in the given area.
    fn forward(&mut self, event: &Event, content: Area) -> EventResult {
        let active = self.active_index();
        match self.tabs.get_mut(active) {
            Some(view) => view.on_event(event.relativized(content.offset)),
            None => EventResult::Ignored,
        }
    }

    fn handle_event(&mut self, event: Event) -> EventResult {
        // The focus is taken away, but it comes back to the same part.
        if event == Event::FocusLost {
            let result = self.lost_content();
            self.held = false;
            return result;
        }

        if self.tabs.is_empty() {
            return EventResult::Ignored;
        }

        if let Event::Mouse { .. } = event {
            return self.mouse_event(&event);
        }

        match self.focus_part() {
            Some(Part::Bar) => self.bar_event(&event),
            // Only a view that has the focus gets the keys. If no part has it, the active view
            // refused it.
            Some(Part::Content) if self.held => {
                let (_, content) = self.areas(self.last_size);
                match self.forward(&event, content) {
                    EventResult::Ignored => self.unused_content_event(&event),
                    result => result,
                }
            }
            _ => EventResult::Ignored,
        }
    }
}

/// A view showing the titles of a [`TabView`], to place somewhere else than next to its content.
///
/// Get one with [`TabView::detached_bar`]. The `TabView` keeps the views of the tabs and only
/// draws the active one; the `TabBar` shows the titles and lets the user pick the active tab.
/// Both share the same state: they always agree on the titles and on the active tab, without
/// any call from the application.
///
/// ```rust
/// use cursive_core::direction::Orientation;
/// use cursive_core::views::{LinearLayout, Panel, TabView, TextView};
///
/// let mut pages = TabView::new()
///     .tab("Overview", TextView::new("Everything is fine."))
///     .tab("Logs", TextView::new("Nothing to report."));
///
/// // A column of titles in a panel, next to the content.
/// let sidebar = Panel::new(pages.detached_bar()).title("Pages");
/// let layout = LinearLayout::horizontal().child(sidebar).child(pages);
///
/// // Another bar for the same tabs, in a line.
/// # let mut pages = TabView::new().tab("A", TextView::new("a")).tab("B", TextView::new("b"));
/// let line = pages.detached_bar().orientation(Orientation::Horizontal);
/// ```
///
/// A `TabBar` is vertical by default: one title per line, like in a sidebar.
///
/// # Focus and keys
///
/// The bar can take the focus when there is more than one tab.
///
/// * In a vertical bar, `Up` and `Down` select the previous and the next tab. In a horizontal
///   bar, these are `Left` and `Right`. They do not wrap around: at the ends the key is ignored,
///   so that a parent can move the focus elsewhere. Other arrow keys are ignored too.
/// * `Home` and `End` select the first and the last tab.
/// * A mouse press on a title selects it (and the parent gives the focus to the bar).
///
/// The active title uses [`TAB_ACTIVE`] when the bar has the focus, and [`TAB_INACTIVE`]
/// when it does not.
///
/// # Size
///
/// Like a list, the bar asks for the room of all its titles, whatever the space it is offered:
/// a parent sharing its space between its children does not squeeze it. A vertical bar is as
/// wide as the widest title, plus padding, and as tall as the number of titles. If it still gets
/// less room, it scrolls to keep the active title in view.
///
/// # Callbacks
///
/// When the user changes the tab with this bar, the [`on_change`](TabView::set_on_change)
/// callback of the `TabView` is called first, then the one of this bar
/// ([`set_on_change`](Self::set_on_change)). Neither is called for changes made in code.
///
/// # Lifetime
///
/// The bar does not own the `TabView`. If the `TabView` is dropped, the bar keeps drawing the
/// titles it last knew, but it cannot take the focus and does not react to events.
///
/// There is no blueprint for this view: it needs a [`TabView`] to share its state with.
pub struct TabBar {
    shared: SharedState,
    orientation: Orientation,
    align: HAlign,
    on_change: Option<Arc<ChangeCallback>>,

    // Size given by the last call to `layout`.
    last_size: Vec2,
    // The titles key used by the last call to `layout`.
    laid_out: u64,
    structure: u64,
    invalidated: bool,
}

impl TabBar {
    fn with_shared(shared: SharedState) -> Self {
        TabBar {
            shared,
            orientation: Orientation::Vertical,
            align: HAlign::Left,
            on_change: None,
            last_size: Vec2::zero(),
            laid_out: 0,
            structure: fresh_layout_key(),
            invalidated: true,
        }
    }

    /// Sets the direction of the titles.
    ///
    /// With [`Orientation::Vertical`] (the default), there is one title per line. With
    /// [`Orientation::Horizontal`], the titles are side by side.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        if self.orientation != orientation {
            self.orientation = orientation;
            self.structure = fresh_layout_key();
            self.invalidated = true;
        }
    }

    /// Sets the direction of the titles.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn orientation(self, orientation: Orientation) -> Self {
        crate::With::with(self, |s| s.set_orientation(orientation))
    }

    /// Sets how the titles are aligned in a horizontal bar.
    ///
    /// This only matters when the titles do not use the whole width. Titles in a vertical bar
    /// are not affected. The default is [`HAlign::Left`].
    pub fn set_align(&mut self, align: HAlign) {
        self.align = align;
    }

    /// Sets how the titles are aligned in a horizontal bar.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn align(self, align: HAlign) -> Self {
        crate::With::with(self, |s| s.set_align(align))
    }

    /// Sets a callback to run when the user changes the active tab with this bar.
    ///
    /// It receives the index of the new active tab. It runs after the
    /// [`on_change`](TabView::set_on_change) callback of the `TabView`.
    ///
    /// It is not called for changes made in code, or in another bar.
    pub fn set_on_change<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, usize) + 'static + Send + Sync,
    {
        self.on_change = Some(Arc::new(callback));
    }

    /// Sets a callback to run when the user changes the active tab with this bar.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_change<F>(self, callback: F) -> Self
    where
        F: Fn(&mut Cursive, usize) + 'static + Send + Sync,
    {
        crate::With::with(self, |s| s.set_on_change(callback))
    }

    fn style(&self) -> BarLayout {
        BarLayout {
            orientation: self.orientation,
            align: self.align,
        }
    }

    // Whether the bar can be used: the `TabView` is there, and there is something to choose.
    fn usable(&self) -> bool {
        let shared = self.shared.lock();
        shared.owner_alive && shared.titles.len() > 1
    }

    fn switch(&self, index: usize) -> EventResult {
        switch_to(&self.shared, index, self.on_change.clone())
    }
}

impl View for TabBar {
    fn draw(&self, printer: &Printer) {
        with_titles(&self.shared, |titles, active| {
            self.style().draw(printer, titles, active);
        });
    }

    fn needs_relayout(&self) -> bool {
        refresh_live(&self.shared);
        self.invalidated || self.shared.lock().titles_key != self.laid_out
    }

    fn layout_key(&self) -> u64 {
        refresh_live(&self.shared);
        combine_layout_key(
            layout_key_seed::<Self>(),
            &(self.structure, self.shared.lock().titles_key),
        )
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        // Like a list, the bar asks for all its titles, whatever the space it is offered: a
        // parent sharing the space between its children should not shrink it. If it still gets
        // less, it scrolls to keep the active title in view.
        refresh_live(&self.shared);
        let style = self.style();
        let (thickness, length) = with_titles(&self.shared, |titles, _| {
            (style.thickness(titles), style.titles_length(titles))
        });
        if style.horizontal() {
            Vec2::new(length, thickness)
        } else {
            Vec2::new(thickness, length)
        }
    }

    fn layout(&mut self, size: Vec2) {
        refresh_live(&self.shared);
        self.last_size = size;
        self.invalidated = false;
        self.laid_out = self.shared.lock().titles_key;
    }

    fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
        if self.usable() {
            Ok(EventResult::consumed())
        } else {
            Err(CannotFocus)
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        let (active, len, alive) = {
            let shared = self.shared.lock();
            (shared.active, shared.titles.len(), shared.owner_alive)
        };
        if !alive || len == 0 {
            return EventResult::Ignored;
        }
        let style = self.style();

        match event {
            Event::Mouse {
                offset,
                position,
                event: MouseEvent::Press(MouseButton::Left),
            } => {
                let Some(position) = position.checked_sub(offset) else {
                    return EventResult::Ignored;
                };
                if !Area::new((0, 0), self.last_size).contains(position) {
                    return EventResult::Ignored;
                }
                let title = with_titles(&self.shared, |titles, _| {
                    style.title_at(titles, active, self.last_size, position)
                });
                match title {
                    Some(index) => self.switch(index),
                    None => EventResult::consumed(),
                }
            }
            Event::Mouse { .. } => EventResult::Ignored,
            _ => match style.key(&event, active, len) {
                BarKey::Switch(index) => self.switch(index),
                BarKey::Ignored | BarKey::Other => EventResult::Ignored,
            },
        }
    }
}

impl View for TabView {
    fn draw(&self, printer: &Printer) {
        if self.tabs.is_empty() {
            return;
        }

        let part = self.focus_part();
        let (bar, content) = self.areas(printer.size);
        let active = self.active_index();
        if !bar.is_empty() {
            let printer = printer
                .offset(bar.offset)
                .cropped(bar.size)
                .focused(part == Some(Part::Bar));
            with_titles(&self.shared, |titles, active| {
                self.bar_style().draw(&printer, titles, active);
            });
        }
        if !content.is_empty() {
            let printer = printer
                .offset(content.offset)
                .cropped(content.size)
                .focused(self.content_focused());
            self.tabs[active].draw(&printer);
        }
    }

    fn needs_relayout(&self) -> bool {
        refresh_live(&self.shared);
        self.invalidated
            || self.shared_key() != self.laid_out
            || self
                .tabs
                .get(self.active_index())
                .is_some_and(|view| view.needs_relayout())
    }

    fn layout_key(&self) -> u64 {
        refresh_live(&self.shared);
        let (titles_key, active) = self.shared_key();
        combine_layout_key(
            layout_key_seed::<Self>(),
            &(
                self.structure,
                titles_key,
                active,
                self.tabs.get(active).map(|view| view.layout_key()),
            ),
        )
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        refresh_live(&self.shared);
        if self.tabs.is_empty() {
            return Vec2::zero();
        }

        let thickness = self.bar_thickness();
        let titles = with_titles(&self.shared, |titles, _| {
            self.bar_style().titles_length(titles)
        });
        let active = self.active_index();
        let view = &mut self.tabs[active];

        if self.placement.titles_axis() == Orientation::Horizontal {
            let thickness = thickness.min(constraint.y);
            let inner = view.required_size(Vec2::new(constraint.x, constraint.y - thickness));
            // Try to show all the titles, if we are allowed to.
            let bar = if thickness > 0 {
                titles.min(constraint.x)
            } else {
                0
            };
            Vec2::new(inner.x.max(bar), inner.y + thickness)
        } else {
            let thickness = thickness.min(constraint.x);
            let inner = view.required_size(Vec2::new(constraint.x - thickness, constraint.y));
            let bar = if thickness > 0 {
                titles.min(constraint.y)
            } else {
                0
            };
            Vec2::new(inner.x + thickness, inner.y.max(bar))
        }
    }

    fn layout(&mut self, size: Vec2) {
        refresh_live(&self.shared);
        self.sync();
        self.last_size = size;
        self.invalidated = false;
        self.laid_out = self.shared_key();
        let (_, content) = self.areas(size);
        let active = self.active_index();
        if let Some(view) = self.tabs.get_mut(active) {
            view.layout(content.size);
        }
    }

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        if self.tabs.is_empty() {
            return Err(CannotFocus);
        }
        self.sync();

        // Start with the part nearest to where the focus comes from.
        let parts = self.parts();
        let candidates = match source.relative(self.placement.separation_axis()) {
            Some(Relative::Front) => parts,
            Some(Relative::Back) => [parts[1], parts[0]],
            None => {
                // From the side, or unspecified: keep the current one.
                let current = self.focus_part().unwrap_or(Part::Bar);
                let other = if current == Part::Bar {
                    Part::Content
                } else {
                    Part::Bar
                };
                [current, other]
            }
        };

        let active = self.active_index();
        for part in candidates {
            match part {
                Part::Bar if self.bar_focusable() => {
                    let lost = self.lost_content();
                    self.give_focus(Part::Bar);
                    return Ok(lost.and(EventResult::consumed()));
                }
                Part::Bar => (),
                Part::Content => {
                    if let Ok(result) = self.tabs[active].take_focus(source) {
                        self.give_focus(Part::Content);
                        return Ok(result);
                    }
                }
            }
        }

        Err(CannotFocus)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        self.sync();
        let result = self.handle_event(event);
        self.deliver_pending(result)
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        for view in &mut self.tabs {
            view.call_on_any(selector, callback);
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.sync();
        for i in 0..self.tabs.len() {
            if let Ok(result) = self.tabs[i].focus_view(selector) {
                // Leave the previous view, if it had the focus.
                let active = self.active_index();
                let lost = if i == active {
                    EventResult::Ignored
                } else {
                    self.lost_content()
                };
                if i != active {
                    self.shared.lock().active = i;
                    self.seen_active = i;
                    self.invalidate();
                }
                self.give_focus(Part::Content);
                return Ok(lost.and(result));
            }
        }
        Err(ViewNotFound)
    }

    fn important_area(&self, size: Vec2) -> Rect {
        if self.tabs.is_empty() {
            return Rect::from_size(Vec2::zero(), size);
        }

        let (bar, content) = self.areas(size);
        match self.focus_part() {
            Some(Part::Bar) => Rect::from_size(bar.offset, bar.size),
            _ => self.tabs[self.active_index()].important_area(content.size) + content.offset,
        }
    }
}

#[cfg(feature = "builder")]
struct TabItem {
    title: StyledString,
    view: BoxedView,
}

#[cfg(feature = "builder")]
impl crate::builder::Resolvable for TabItem {
    fn from_config(
        config: &crate::builder::Config,
        context: &crate::builder::Context,
    ) -> Result<Self, crate::builder::Error> {
        use crate::builder::Error;

        let object = config.as_object().ok_or_else(|| {
            Error::invalid_config("Expected an object with a title and a view", config)
        })?;
        let title = object
            .get("title")
            .ok_or_else(|| Error::invalid_config("Missing tab title", config))?;
        let view = object
            .get("view")
            .ok_or_else(|| Error::invalid_config("Missing tab view", config))?;

        Ok(TabItem {
            title: context.resolve(title)?,
            view: context.resolve(view)?,
        })
    }
}

crate::manual_blueprint!(TabView, |config, context| {
    use crate::builder::Error;

    let mut view = TabView::new();

    let tabs: Option<Vec<TabItem>> = context.resolve_as_config(&config["tabs"])?;
    for tab in tabs.into_iter().flatten() {
        view.add_tab(tab.title, tab.view);
    }

    if let Some(placement) = config.get("bar_placement") {
        view.set_bar_placement(context.resolve(placement)?);
    }
    if let Some(align) = config.get("bar_align") {
        view.set_bar_align(context.resolve(align)?);
    }
    if let Some(show_bar) = config.get("show_bar") {
        view.set_show_bar(context.resolve(show_bar)?);
    }
    if let Some(active) = config.get("active") {
        let active = context.resolve(active)?;
        view.set_active(active).map_err(|_| Error::InvalidConfig {
            message: "TabView.active must be smaller than the number of tabs.".into(),
            config: config.clone(),
        })?;
    }

    Ok(view)
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::style::{Color, ConcreteStyle, Effect, Style};
    use crate::theme::Theme;
    use crate::view::{Finder, Nameable};
    use crate::views::{EditView, LinearLayout, TextView};
    use parking_lot::RwLock;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// What a `Probe` saw.
    #[derive(Default)]
    struct Seen {
        events: Mutex<Vec<Event>>,
        sources: Mutex<Vec<Direction>>,
        required: AtomicUsize,
        layouts: AtomicUsize,
        draws: AtomicUsize,
    }

    /// A view filling its area with a letter, and recording what happens to it.
    struct Probe {
        letter: &'static str,
        size: Vec2,
        focusable: bool,
        consume: bool,
        seen: Arc<Seen>,
    }

    impl View for Probe {
        fn draw(&self, printer: &Printer) {
            self.seen.draws.fetch_add(1, Ordering::Relaxed);
            for y in 0..printer.size.y {
                printer.print_hline((0, y), printer.size.x, self.letter);
            }
        }

        fn needs_relayout(&self) -> bool {
            false
        }

        fn required_size(&mut self, _: Vec2) -> Vec2 {
            self.seen.required.fetch_add(1, Ordering::Relaxed);
            self.size
        }

        fn layout(&mut self, _: Vec2) {
            self.seen.layouts.fetch_add(1, Ordering::Relaxed);
        }

        fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
            self.seen.sources.lock().unwrap().push(source);
            if self.focusable {
                Ok(EventResult::consumed())
            } else {
                Err(CannotFocus)
            }
        }

        fn on_event(&mut self, event: Event) -> EventResult {
            self.seen.events.lock().unwrap().push(event);
            if self.consume {
                EventResult::consumed()
            } else {
                EventResult::Ignored
            }
        }
    }

    fn probe(letter: &'static str, focusable: bool) -> (Probe, Arc<Seen>) {
        let seen = Arc::new(Seen::default());
        let probe = Probe {
            letter,
            size: Vec2::new(2, 2),
            focusable,
            consume: false,
            seen: Arc::clone(&seen),
        };
        (probe, seen)
    }

    /// Three tabs titled "One", "Two" and "Six", with focusable probes "a", "b" and "c".
    fn view(placement: TabPlacement) -> TabView {
        let mut view = TabView::new().bar_placement(placement);
        for (title, letter) in [("One", "a"), ("Two", "b"), ("Six", "c")] {
            view.add_tab(title, probe(letter, true).0);
        }
        view
    }

    fn buffer_for(
        view: &mut TabView,
        size: impl Into<Vec2>,
        focused: bool,
        theme: &Theme,
    ) -> RwLock<PrintBuffer> {
        let size = size.into();
        view.layout(size);
        // Room around the view, to see if anything goes outside.
        let mut buffer = PrintBuffer::new();
        buffer.resize(size + (3, 2));
        let buffer = RwLock::new(buffer);
        view.draw(&Printer::new(size, theme, &buffer).focused(focused));
        buffer
    }

    /// Text of each line, with `.` for cells that were not printed and `_` for spaces.
    ///
    /// Panics if something was printed outside of `size`.
    fn render(view: &mut TabView, size: impl Into<Vec2>) -> Vec<String> {
        let size = size.into();
        let buffer = buffer_for(view, size, true, &Theme::retro());
        let buffer = buffer.read();
        let full = buffer.size();
        let rows: Vec<Vec<&str>> = (0..full.y)
            .map(|y| {
                (0..full.x)
                    .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                        None | Some("") => ".",
                        Some(" ") => "_",
                        Some(text) => text,
                    })
                    .collect()
            })
            .collect();
        for (y, row) in rows.iter().enumerate() {
            for (x, text) in row.iter().enumerate() {
                assert!(
                    (x < size.x && y < size.y) || *text == ".",
                    "{text:?} was printed outside the view, at ({x}, {y})"
                );
            }
        }
        rows[..size.y]
            .iter()
            .map(|row| row[..size.x].concat())
            .collect()
    }

    /// Sends an event, runs the callback, and tells if it was consumed.
    fn send(view: &mut TabView, event: impl Into<Event>) -> bool {
        let result = view.on_event(event.into());
        let consumed = result.is_consumed();
        result.process(&mut Cursive::new());
        consumed
    }

    fn press(offset: impl Into<Vec2>, position: impl Into<Vec2>) -> Event {
        Event::Mouse {
            offset: offset.into(),
            position: position.into(),
            event: MouseEvent::Press(MouseButton::Left),
        }
    }

    fn recording(view: TabView) -> (TabView, Arc<Mutex<Vec<usize>>>) {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&changes);
        let view = view.on_change(move |_, index| log.lock().unwrap().push(index));
        (view, changes)
    }

    fn letter(view: &TabView, index: usize) -> &'static str {
        view.get_tab(index).unwrap().get::<Probe>().unwrap().letter
    }

    fn cell_style(buffer: &RwLock<PrintBuffer>, x: usize, y: usize) -> ConcreteStyle {
        buffer.read().cell_style(Vec2::new(x, y)).unwrap()
    }

    /// The style `PaletteStyle` gives to a printed cell.
    fn palette_style(style: PaletteStyle, theme: &Theme) -> ConcreteStyle {
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(1, 1));
        let buffer = RwLock::new(buffer);
        Printer::new((1, 1), theme, &buffer)
            .with_style(style, |printer| printer.print((0, 0), " "));
        cell_style(&buffer, 0, 0)
    }

    #[test]
    fn top_bar() {
        let mut view = view(TabPlacement::Top);
        assert_eq!(
            render(&mut view, (16, 4)),
            [
                "_One__Two__Six_.",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
            ]
        );

        view.set_active(2).unwrap();
        assert_eq!(render(&mut view, (16, 2))[1], "cccccccccccccccc");
    }

    #[test]
    fn bottom_bar() {
        let mut view = view(TabPlacement::Bottom);
        assert_eq!(
            render(&mut view, (16, 4)),
            [
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "_One__Two__Six_.",
            ]
        );
    }

    #[test]
    fn left_bar() {
        let mut view = view(TabPlacement::Left);
        assert_eq!(
            render(&mut view, (12, 4)),
            [
                "_One_aaaaaaa",
                "_Two_aaaaaaa",
                "_Six_aaaaaaa",
                ".....aaaaaaa",
            ]
        );
    }

    #[test]
    fn right_bar() {
        let mut view = view(TabPlacement::Right);
        assert_eq!(
            render(&mut view, (12, 4)),
            [
                "aaaaaaa_One_",
                "aaaaaaa_Two_",
                "aaaaaaa_Six_",
                "aaaaaaa.....",
            ]
        );
    }

    #[test]
    fn column_is_as_wide_as_the_widest_title() {
        let mut view = TabView::new()
            .bar_placement(TabPlacement::Left)
            .tab("A", probe("a", true).0)
            .tab("Long", probe("b", true).0);
        assert_eq!(render(&mut view, (9, 2)), ["_A____aaa", "_Long_aaa"]);
        // The bar and the view next to it.
        assert_eq!(view.required_size(Vec2::new(40, 40)), Vec2::new(8, 2));
    }

    #[test]
    fn bar_alignment() {
        let mut view = view(TabPlacement::Top);
        assert_eq!(render(&mut view, (20, 1))[0], "_One__Two__Six_.....");

        view.set_bar_align(HAlign::Center);
        assert_eq!(render(&mut view, (20, 1))[0], ".._One__Two__Six_...");

        view.set_bar_align(HAlign::Right);
        assert_eq!(render(&mut view, (20, 1))[0], "....._One__Two__Six_");

        // The alignment does not matter for a column.
        let mut left = self::view(TabPlacement::Left).bar_align(HAlign::Right);
        assert_eq!(render(&mut left, (8, 1))[0], "_One_aaa");
    }

    #[test]
    fn required_size() {
        let mut view = view(TabPlacement::Top);
        // The active view is 2x2, the titles take 15 cells.
        assert_eq!(view.required_size(Vec2::new(40, 40)), Vec2::new(15, 3));
        // The titles do not make the view larger than what it is given.
        assert_eq!(view.required_size(Vec2::new(10, 40)), Vec2::new(10, 3));

        view.set_bar_placement(TabPlacement::Right);
        // Three titles in a column of width 5.
        assert_eq!(view.required_size(Vec2::new(40, 40)), Vec2::new(7, 3));
        assert_eq!(view.required_size(Vec2::new(40, 2)), Vec2::new(7, 2));
    }

    #[test]
    fn only_the_active_view_is_used() {
        let mut view = TabView::new();
        let mut seen = Vec::new();
        for letter in ["a", "b", "c"] {
            let (probe, state) = probe(letter, true);
            view.add_tab(letter, probe);
            seen.push(state);
        }
        view.set_active(1).unwrap();

        render(&mut view, (10, 4));
        let counts: Vec<_> = seen
            .iter()
            .map(|state| {
                (
                    state.required.load(Ordering::Relaxed) > 0,
                    state.layouts.load(Ordering::Relaxed) > 0,
                    state.draws.load(Ordering::Relaxed) > 0,
                )
            })
            .collect();
        view.required_size(Vec2::new(10, 4));
        assert_eq!(
            counts,
            [
                (false, false, false),
                (false, true, true),
                (false, false, false)
            ]
        );
        assert_eq!(seen[0].required.load(Ordering::Relaxed), 0);
        assert_eq!(seen[2].required.load(Ordering::Relaxed), 0);
        assert!(seen[1].required.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn active_title_style() {
        let theme = Theme::retro();
        let highlight = palette_style(PaletteStyle::Highlight, &theme);
        let inactive = palette_style(PaletteStyle::HighlightInactive, &theme);
        let plain = {
            let mut buffer = PrintBuffer::new();
            buffer.resize(Vec2::new(1, 1));
            let buffer = RwLock::new(buffer);
            Printer::new((1, 1), &theme, &buffer).print((0, 0), " ");
            cell_style(&buffer, 0, 0)
        };
        assert_ne!(highlight, inactive);
        assert_ne!(highlight, plain);

        let mut view = view(TabPlacement::Top);
        let focused = buffer_for(&mut view, (16, 3), true, &theme);
        // The whole padded title: " One ".
        for x in 0..5 {
            assert_eq!(cell_style(&focused, x, 0), highlight);
        }
        // The other titles use the current style.
        for x in 5..15 {
            assert_eq!(cell_style(&focused, x, 0), plain);
        }

        let unfocused = buffer_for(&mut view, (16, 3), false, &theme);
        for x in 0..5 {
            assert_eq!(cell_style(&unfocused, x, 0), inactive);
        }
        assert_eq!(cell_style(&unfocused, 6, 0), plain);

        // Same in a column, where the whole width is highlighted.
        let mut view = self::view(TabPlacement::Left);
        view.set_active(1).unwrap();
        let buffer = buffer_for(&mut view, (8, 3), true, &theme);
        for x in 0..5 {
            assert_eq!(cell_style(&buffer, x, 1), highlight);
            assert_eq!(cell_style(&buffer, x, 0), plain);
        }
    }

    #[test]
    fn a_bar_that_cannot_be_focused_is_unfocused() {
        let theme = Theme::retro();
        let inactive = palette_style(PaletteStyle::HighlightInactive, &theme);
        let mut view = TabView::new().tab("One", probe("a", true).0);
        let buffer = buffer_for(&mut view, (8, 2), true, &theme);
        assert_eq!(cell_style(&buffer, 1, 0), inactive);
    }

    #[test]
    fn style_keys() {
        let mut theme = Theme::retro();
        let active = Style::from(Color::Rgb(1, 2, 3));
        let inactive = Style::from(Color::Rgb(4, 5, 6));
        theme.palette.set_custom_style(&TAB_ACTIVE, active);
        theme.palette.set_style("tabs.inactive", inactive);

        let mut view = view(TabPlacement::Top);
        let focused = buffer_for(&mut view, (16, 3), true, &theme);
        assert_eq!(cell_style(&focused, 1, 0).color.front, Color::Rgb(1, 2, 3));
        let unfocused = buffer_for(&mut view, (16, 3), false, &theme);
        assert_eq!(
            cell_style(&unfocused, 1, 0).color.front,
            Color::Rgb(4, 5, 6)
        );

        // Only one key defined: the other falls back to the palette.
        let mut theme = Theme::retro();
        theme.palette.set_custom_style(&TAB_ACTIVE, active);
        let unfocused = buffer_for(&mut view, (16, 3), false, &theme);
        assert_eq!(
            cell_style(&unfocused, 1, 0),
            palette_style(PaletteStyle::HighlightInactive, &theme)
        );
    }

    #[test]
    fn styled_titles() {
        let theme = Theme::retro();
        let highlight = palette_style(PaletteStyle::Highlight, &theme);

        let red = Color::Rgb(255, 0, 0);
        let title = StyledString::styled("One", Style::from(red));
        let bold = StyledString::styled("Two", Style::from(Effect::Bold));
        let mut view = TabView::new()
            .tab(title, probe("a", true).0)
            .tab(bold, probe("b", true).0);

        // Inactive: the title keeps its own style.
        view.set_active(1).unwrap();
        let buffer = buffer_for(&mut view, (12, 2), true, &theme);
        assert_eq!(cell_style(&buffer, 2, 0).color.front, red);
        assert!(!cell_style(&buffer, 2, 0).effects.contains(Effect::Bold));

        // Active: where they conflict (the colors), the active style wins.
        // What does not conflict (an effect) is kept.
        view.set_active(0).unwrap();
        let buffer = buffer_for(&mut view, (12, 2), true, &theme);
        assert_eq!(cell_style(&buffer, 2, 0).color, highlight.color);
        view.set_active(1).unwrap();
        let buffer = buffer_for(&mut view, (12, 2), true, &theme);
        let style = cell_style(&buffer, 7, 0);
        assert_eq!(style.color, highlight.color);
        assert!(style.effects.contains(Effect::Bold));
        // The padding only has the active style.
        assert!(!cell_style(&buffer, 5, 0).effects.contains(Effect::Bold));
    }

    #[test]
    fn titles_wider_than_the_bar() {
        let mut view = TabView::new();
        for (title, letter) in [("AAAA", "a"), ("BBBB", "b"), ("CCCC", "c")] {
            view.add_tab(title, probe(letter, true).0);
        }

        // The bar is 10 cells wide, the titles take 18.
        // Rendering fails if anything is drawn outside.
        assert_eq!(render(&mut view, (10, 2))[0], "_AAAA__BBB");

        view.set_active(1).unwrap();
        // Starting at the first title that keeps the active one in view.
        assert_eq!(render(&mut view, (10, 2))[0], "_BBBB__CCC");

        view.set_active(2).unwrap();
        assert_eq!(render(&mut view, (10, 2))[0], "_CCCC_....");

        // Narrower than the active title.
        assert_eq!(render(&mut view, (3, 2))[0], "_CC");
        assert_eq!(render(&mut view, (0, 0)), Vec::<String>::new());
        assert_eq!(render(&mut view, (1, 2)), ["_", "c"]);
    }

    #[test]
    fn column_taller_than_the_view() {
        let mut view = view(TabPlacement::Left);
        view.set_active(2).unwrap();
        // Three titles, two lines: the active one is still visible.
        assert_eq!(render(&mut view, (8, 2)), ["_Two_ccc", "_Six_ccc"]);
        view.set_active(0).unwrap();
        assert_eq!(render(&mut view, (8, 2)), ["_One_aaa", "_Two_aaa"]);
        assert_eq!(render(&mut view, (8, 1)), ["_One_aaa"]);
    }

    #[test]
    fn small_sizes_stay_inside() {
        for placement in [
            TabPlacement::Top,
            TabPlacement::Bottom,
            TabPlacement::Left,
            TabPlacement::Right,
        ] {
            for width in 0..9 {
                for height in 0..4 {
                    let mut view = view(placement);
                    render(&mut view, (width, height));
                    view.set_show_bar(false);
                }
            }
        }
    }

    fn switch_keys(placement: TabPlacement) -> (Key, Key) {
        match placement {
            TabPlacement::Top | TabPlacement::Bottom => (Key::Left, Key::Right),
            TabPlacement::Left | TabPlacement::Right => (Key::Up, Key::Down),
        }
    }

    #[test]
    fn keyboard_switching() {
        for placement in [
            TabPlacement::Top,
            TabPlacement::Bottom,
            TabPlacement::Left,
            TabPlacement::Right,
        ] {
            let (previous, next) = switch_keys(placement);
            let (mut view, changes) = recording(view(placement));
            view.take_focus(Direction::none()).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Bar));

            // The start of the bar: nothing before.
            assert!(!send(&mut view, previous), "{placement:?}");
            assert!(send(&mut view, next));
            assert!(send(&mut view, next));
            assert_eq!(view.active(), Some(2));
            // The end: no wrap-around.
            assert!(!send(&mut view, next), "{placement:?}");
            assert_eq!(view.active(), Some(2));
            assert!(send(&mut view, previous));
            assert!(send(&mut view, previous));
            assert!(!send(&mut view, previous));
            assert_eq!(view.active(), Some(0));

            assert!(send(&mut view, Key::End));
            assert_eq!(view.active(), Some(2));
            assert!(send(&mut view, Key::Home));
            assert_eq!(view.active(), Some(0));
            // Already there: not a change, and the key is left to the parent.
            assert!(!send(&mut view, Key::Home));

            assert_eq!(
                *changes.lock().unwrap(),
                [1, 2, 1, 0, 2, 0],
                "{placement:?}"
            );
            // Switching does not move the focus into the view.
            assert_eq!(view.focus_part(), Some(Part::Bar));
        }
    }

    #[test]
    fn on_change_is_not_called_for_code() {
        let (mut view, changes) = recording(view(TabPlacement::Top));
        view.set_active(2).unwrap();
        view.remove_tab(0);
        view.insert_tab(0, "New", probe("n", true).0);
        view.set_title(0, "Newer");
        assert!(changes.lock().unwrap().is_empty());

        view.take_focus(Direction::none()).unwrap();
        send(&mut view, Key::Left);
        assert_eq!(*changes.lock().unwrap(), [1]);
    }

    #[test]
    fn keys_the_bar_does_not_use_are_ignored() {
        let mut view = view(TabPlacement::Top);
        view.take_focus(Direction::none()).unwrap();
        // Up and Down along the other axis: for the parent.
        assert!(!send(&mut view, Key::Up));
        assert!(!send(&mut view, Event::Char('x')));
        assert!(!send(&mut view, Key::Enter));
    }

    #[test]
    fn focus_moves_between_bar_and_content() {
        // (placement, key to the view, key to the bar)
        for (placement, to_view, to_bar) in [
            (TabPlacement::Top, Key::Down, Key::Up),
            (TabPlacement::Bottom, Key::Up, Key::Down),
            (TabPlacement::Left, Key::Right, Key::Left),
            (TabPlacement::Right, Key::Left, Key::Right),
        ] {
            let mut view = view(placement);
            view.layout(Vec2::new(12, 6));
            view.take_focus(Direction::none()).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Bar), "{placement:?}");

            // Not along the axis between the bar and the view.
            assert!(!send(&mut view, to_bar), "{placement:?}");
            assert_eq!(view.focus_part(), Some(Part::Bar));

            assert!(send(&mut view, to_view), "{placement:?}");
            assert_eq!(view.focus_part(), Some(Part::Content));
            // The content is still the same one.
            assert_eq!(view.active(), Some(0));

            // The content ignores the keys: they go to the parent, or back to the bar.
            assert!(!send(&mut view, to_view), "{placement:?}");
            assert!(send(&mut view, to_bar), "{placement:?}");
            assert_eq!(view.focus_part(), Some(Part::Bar));
        }
    }

    #[test]
    fn keys_ignored_by_content_go_to_the_parent() {
        let mut view = view(TabPlacement::Top);
        view.layout(Vec2::new(12, 6));
        view.take_focus(Direction::down()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert!(!send(&mut view, Key::Left));
        assert!(!send(&mut view, Key::Right));
        assert!(!send(&mut view, Key::Down));
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert_eq!(view.active(), Some(0));
    }

    #[test]
    fn content_gets_the_events_it_has_focus_for() {
        let (mut content, seen) = probe("a", true);
        content.consume = true;
        let mut view = TabView::new()
            .tab("One", content)
            .tab("Two", probe("b", true).0);
        view.layout(Vec2::new(12, 6));
        view.take_focus(Direction::down()).unwrap();

        // The content uses this one, so the focus stays.
        assert!(send(&mut view, Key::Up));
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert!(matches!(
            seen.events.lock().unwrap()[..],
            [Event::Key(Key::Up)]
        ));

        // With the focus on the bar, the content does not see keys.
        view.take_focus(Direction::up()).unwrap();
        let before = seen.events.lock().unwrap().len();
        send(&mut view, Key::Down);
        // Down went from the bar to the view.
        assert_eq!(seen.events.lock().unwrap().len(), before);
        assert_eq!(view.focus_part(), Some(Part::Content));
    }

    #[test]
    fn content_loses_the_focus() {
        let (mut content, seen) = probe("a", true);
        content.consume = false;
        let mut view = TabView::new()
            .tab("One", content)
            .tab("Two", probe("b", true).0);
        view.layout(Vec2::new(12, 6));
        view.take_focus(Direction::down()).unwrap();
        seen.events.lock().unwrap().clear();

        send(&mut view, Key::Up);
        assert_eq!(view.focus_part(), Some(Part::Bar));
        let events = seen.events.lock().unwrap();
        assert!(matches!(
            events[..],
            [Event::Key(Key::Up), Event::FocusLost]
        ));
    }

    #[test]
    fn focus_with_tab_keys() {
        let mut view = view(TabPlacement::Top);
        view.layout(Vec2::new(12, 6));
        view.take_focus(Direction::up()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert!(send(&mut view, Key::Tab));
        assert_eq!(view.focus_part(), Some(Part::Content));
        // Last part: the parent decides.
        assert!(!send(&mut view, Key::Tab));
        assert!(send(&mut view, Event::Shift(Key::Tab)));
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert!(!send(&mut view, Event::Shift(Key::Tab)));

        // The bar is the last part.
        let mut view = self::view(TabPlacement::Bottom);
        view.layout(Vec2::new(12, 6));
        view.take_focus(Direction::up()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert!(send(&mut view, Key::Tab));
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert!(!send(&mut view, Key::Tab));
        assert!(send(&mut view, Event::Shift(Key::Tab)));
        assert_eq!(view.focus_part(), Some(Part::Content));
    }

    #[test]
    fn tab_does_not_enter_a_view_that_cannot_focus() {
        let mut view = TabView::new()
            .tab("One", probe("a", false).0)
            .tab("Two", probe("b", false).0);
        view.take_focus(Direction::none()).unwrap();
        assert!(!send(&mut view, Key::Tab));
        assert!(!send(&mut view, Key::Down));
        assert_eq!(view.focus_part(), Some(Part::Bar));
    }

    #[test]
    fn take_focus_from_each_direction() {
        let up = Direction::up();
        let down = Direction::down();
        let left = Direction::left();
        let right = Direction::right();
        // Coming from above, the top bar comes first.
        // (placement, source, expected part)
        let cases = [
            (TabPlacement::Top, up, Part::Bar),
            (TabPlacement::Top, down, Part::Content),
            (TabPlacement::Top, Direction::front(), Part::Bar),
            (TabPlacement::Top, Direction::back(), Part::Content),
            (TabPlacement::Bottom, up, Part::Content),
            (TabPlacement::Bottom, down, Part::Bar),
            (TabPlacement::Left, left, Part::Bar),
            (TabPlacement::Left, right, Part::Content),
            (TabPlacement::Right, left, Part::Content),
            (TabPlacement::Right, right, Part::Bar),
        ];
        for (placement, source, expected) in cases {
            let mut view = view(placement);
            view.take_focus(source).unwrap();
            assert_eq!(
                view.focus_part(),
                Some(expected),
                "{placement:?} {source:?}"
            );
        }

        // From the side, or from nowhere: keep the part that had the focus.
        for (placement, source) in [
            (TabPlacement::Top, left),
            (TabPlacement::Bottom, right),
            (TabPlacement::Left, up),
            (TabPlacement::Right, Direction::none()),
        ] {
            let mut view = view(placement);
            view.take_focus(Direction::none()).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Bar));
            view.take_focus(source).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Bar));
            view.layout(Vec2::new(12, 6));
            send(
                &mut view,
                match placement {
                    TabPlacement::Top => Key::Down,
                    TabPlacement::Bottom => Key::Up,
                    TabPlacement::Left => Key::Right,
                    _ => Key::Left,
                },
            );
            assert_eq!(view.focus_part(), Some(Part::Content));
            view.take_focus(source).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Content));
        }
    }

    #[test]
    fn take_focus_falls_back_to_the_other_part() {
        // A view that cannot take the focus: the bar is the only choice.
        for source in [Direction::up(), Direction::down()] {
            let mut view = TabView::new()
                .tab("One", probe("a", false).0)
                .tab("Two", probe("b", false).0);
            view.take_focus(source).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Bar));
        }

        // Only one tab, or a hidden bar: only the view.
        for source in [Direction::up(), Direction::down()] {
            let mut view = TabView::new().tab("One", probe("a", true).0);
            view.take_focus(source).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Content));

            let mut view = TabView::new()
                .tab("One", probe("a", true).0)
                .tab("Two", probe("b", true).0)
                .show_bar(false);
            view.take_focus(source).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Content));
        }

        // Nothing can take the focus.
        let mut view = TabView::new().tab("One", probe("a", false).0);
        assert!(view.take_focus(Direction::none()).is_err());
        let mut view = TabView::new()
            .tab("One", probe("a", false).0)
            .tab("Two", probe("b", false).0)
            .show_bar(false);
        assert!(view.take_focus(Direction::none()).is_err());

        // The source is passed on to the view.
        let (content, seen) = probe("a", true);
        let mut view = TabView::new().tab("One", content);
        view.take_focus(Direction::left()).unwrap();
        assert_eq!(*seen.sources.lock().unwrap(), [Direction::left()]);
    }

    #[test]
    fn mouse_on_titles() {
        // Titles are at 0..5, 5..10 and 10..15.
        let (mut view, changes) = recording(view(TabPlacement::Top));
        view.layout(Vec2::new(16, 4));

        assert!(send(&mut view, press((0, 0), (6, 0))));
        assert_eq!(view.active(), Some(1));
        assert!(send(&mut view, press((0, 0), (14, 0))));
        assert_eq!(view.active(), Some(2));
        assert!(send(&mut view, press((0, 0), (0, 0))));
        assert_eq!(view.active(), Some(0));
        // The bar is focused, and the active title again: not a change.
        assert!(send(&mut view, press((0, 0), (3, 0))));
        assert_eq!(*changes.lock().unwrap(), [1, 2, 0]);

        // The offset of the view is taken into account.
        assert!(send(&mut view, press((10, 5), (17, 5))));
        assert_eq!(view.active(), Some(1));

        // After the last title, in the bar: nothing to select, but the bar is focused.
        view.take_focus(Direction::down()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert!(send(&mut view, press((0, 0), (15, 0))));
        assert_eq!(view.active(), Some(1));
        assert_eq!(view.focus_part(), Some(Part::Bar));
    }

    #[test]
    fn mouse_on_titles_in_each_placement() {
        for (placement, size, positions) in [
            (TabPlacement::Bottom, (16, 4), [(1, 3), (7, 3), (12, 3)]),
            (TabPlacement::Left, (12, 4), [(1, 0), (3, 1), (4, 2)]),
            (TabPlacement::Right, (12, 4), [(7, 0), (9, 1), (11, 2)]),
        ] {
            let (mut view, changes) = recording(view(placement));
            view.layout(size.into());
            for (expected, position) in positions.into_iter().enumerate().rev() {
                assert!(send(&mut view, press((0, 0), position)), "{placement:?}");
                assert_eq!(view.active(), Some(expected), "{placement:?}");
                assert_eq!(view.focus_part(), Some(Part::Bar));
            }
            assert_eq!(*changes.lock().unwrap(), [2, 1, 0], "{placement:?}");
        }
    }

    #[test]
    fn mouse_on_the_scrolled_bar() {
        let mut view = TabView::new();
        for (title, letter) in [("AAAA", "a"), ("BBBB", "b"), ("CCCC", "c")] {
            view.add_tab(title, probe(letter, true).0);
        }
        view.set_active(2).unwrap();
        view.layout(Vec2::new(10, 3));
        // The bar shows the last title first.
        assert!(send(&mut view, press((0, 0), (3, 0))));
        assert_eq!(view.active(), Some(2));
        // Clicks past the end of the bar are not in the bar.
        assert!(!send(&mut view, press((0, 0), (12, 0))));
        assert_eq!(view.active(), Some(2));
    }

    #[test]
    fn mouse_in_the_content() {
        for (placement, content_offset) in [
            (TabPlacement::Top, (0, 1)),
            (TabPlacement::Bottom, (0, 0)),
            (TabPlacement::Left, (5, 0)),
            (TabPlacement::Right, (0, 0)),
        ] {
            let (content, seen) = probe("a", true);
            let mut view = TabView::new()
                .bar_placement(placement)
                .tab("One", content)
                .tab("Two", probe("b", true).0);
            view.layout(Vec2::new(12, 5));
            // Starts on the bar if there is one.
            view.take_focus(Direction::none()).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Bar));

            let position = Vec2::new(content_offset.0 + 1, content_offset.1 + 2);
            // The view itself is at (3, 4).
            let event = press((3, 4), position + (3, 4));
            send(&mut view, event);
            assert_eq!(view.focus_part(), Some(Part::Content), "{placement:?}");

            let events = seen.events.lock().unwrap();
            match events[..] {
                [
                    Event::Mouse {
                        offset,
                        position: forwarded,
                        ..
                    },
                ] => {
                    assert_eq!(offset, Vec2::new(3, 4) + content_offset, "{placement:?}");
                    assert_eq!(forwarded, position + (3, 4));
                    // The content can find where it was hit.
                    assert_eq!(forwarded - offset, Vec2::new(1, 2));
                }
                _ => panic!("expected one mouse event, got {}", events.len()),
            }
        }
    }

    #[test]
    fn mouse_follows_the_focus() {
        let (content, seen) = probe("a", false);
        let mut view = TabView::new()
            .tab("One", content)
            .tab("Two", probe("b", true).0);
        view.layout(Vec2::new(12, 5));
        view.take_focus(Direction::none()).unwrap();

        // The content cannot take the focus, but it gets the click.
        send(&mut view, press((0, 0), (1, 3)));
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert_eq!(seen.events.lock().unwrap().len(), 1);

        // A release does not go to the content if it does not have the focus.
        let release = Event::Mouse {
            offset: Vec2::zero(),
            position: Vec2::new(1, 3),
            event: MouseEvent::Release(MouseButton::Left),
        };
        assert!(!send(&mut view, release.clone()));
        assert_eq!(seen.events.lock().unwrap().len(), 1);

        // Clicking on a title takes the focus back from a focused content.
        let (content, seen) = probe("a", true);
        let mut view = TabView::new()
            .tab("One", content)
            .tab("Two", probe("b", true).0);
        view.layout(Vec2::new(12, 5));
        send(&mut view, press((0, 0), (1, 3)));
        assert_eq!(view.focus_part(), Some(Part::Content));
        // Now the release goes to it.
        send(&mut view, release);
        assert_eq!(seen.events.lock().unwrap().len(), 2);

        seen.events.lock().unwrap().clear();
        send(&mut view, press((0, 0), (7, 0)));
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert_eq!(view.active(), Some(1));
        assert!(matches!(
            seen.events.lock().unwrap()[..],
            [Event::FocusLost]
        ));
    }

    #[test]
    fn mouse_outside_is_ignored() {
        let mut view = view(TabPlacement::Top);
        view.layout(Vec2::new(12, 4));
        // Before the offset of the view.
        assert!(!send(&mut view, press((5, 5), (2, 2))));
        // Past the end of the view.
        assert!(!send(&mut view, press((0, 0), (13, 2))));
        assert!(!send(&mut view, press((0, 0), (3, 7))));
        // Wheel and other buttons on the bar do nothing.
        for event in [MouseEvent::WheelDown, MouseEvent::Press(MouseButton::Right)] {
            let event = Event::Mouse {
                offset: Vec2::zero(),
                position: Vec2::new(7, 0),
                event,
            };
            assert!(!send(&mut view, event));
        }
        assert_eq!(view.active(), Some(0));
    }

    #[test]
    fn important_area() {
        let mut view = view(TabPlacement::Top);
        let size = Vec2::new(12, 6);
        view.layout(size);
        view.take_focus(Direction::up()).unwrap();
        assert_eq!(view.important_area(size), Rect::from_size((0, 0), (12, 1)));
        view.take_focus(Direction::down()).unwrap();
        send(&mut view, Key::Tab);
        // The whole content, below the bar.
        assert_eq!(view.important_area(size), Rect::from_size((0, 1), (12, 5)));

        let mut view = self::view(TabPlacement::Right);
        view.take_focus(Direction::right()).unwrap();
        assert_eq!(view.important_area(size), Rect::from_size((7, 0), (5, 6)));

        assert_eq!(
            TabView::new().important_area(size),
            Rect::from_size((0, 0), size)
        );
    }

    #[test]
    fn important_area_of_the_content_is_moved() {
        struct Corner;
        impl View for Corner {
            fn draw(&self, _: &Printer) {}
            fn important_area(&self, _: Vec2) -> Rect {
                Rect::from_size((1, 1), (1, 1))
            }
            fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
                Ok(EventResult::consumed())
            }
        }

        let mut view = TabView::new()
            .bar_placement(TabPlacement::Left)
            .tab("One", Corner)
            .tab("Two", Corner);
        view.take_focus(Direction::right()).unwrap();
        let size = Vec2::new(12, 6);
        view.layout(size);
        assert_eq!(view.important_area(size), Rect::from_size((6, 1), (1, 1)));
    }

    #[test]
    fn adding_tabs() {
        let mut view = TabView::new();
        assert_eq!(view.active(), None);
        assert!(view.is_empty());

        view.add_tab("One", probe("a", true).0);
        assert_eq!((view.len(), view.active()), (1, Some(0)));
        view.add_tab("Two", probe("b", true).0);
        view.add_tab("Six", probe("c", true).0);
        assert_eq!((view.len(), view.active()), (3, Some(0)));

        // Before the active tab: it stays the same one.
        view.set_active(1).unwrap();
        view.insert_tab(0, "New", probe("n", true).0);
        assert_eq!(view.active(), Some(2));
        assert_eq!(letter(&view, 2), "b");
        // At the active tab: same.
        view.insert_tab(2, "Newer", probe("m", true).0);
        assert_eq!(view.active(), Some(3));
        assert_eq!(letter(&view, 3), "b");
        // After it.
        view.insert_tab(4, "Last", probe("l", true).0);
        assert_eq!(view.active(), Some(3));
        // Past the end: at the end.
        view.insert_tab(100, "End", probe("e", true).0);
        assert_eq!(view.len(), 7);
        assert_eq!(letter(&view, 6), "e");
        assert_eq!(view.active(), Some(3));
        assert!(view.get_tab(7).is_none());
        assert!(view.get_tab_mut(7).is_none());

        // Inserting in an empty view makes it the active one.
        let mut view = TabView::new();
        view.insert_tab(5, "One", probe("a", true).0);
        assert_eq!((view.len(), view.active()), (1, Some(0)));
    }

    #[test]
    fn removing_tabs() {
        let mut view = view(TabPlacement::Top);
        view.add_tab("Four", probe("d", true).0);
        assert_eq!(view.len(), 4);

        // Out of range.
        assert!(view.remove_tab(4).is_none());
        assert_eq!(view.len(), 4);

        // Before the active tab: it stays the same one.
        view.set_active(2).unwrap();
        let removed = view.remove_tab(0).unwrap();
        assert_eq!(removed.get::<Probe>().unwrap().letter, "a");
        assert_eq!(view.active(), Some(1));
        assert_eq!(letter(&view, 1), "c");

        // After it: nothing changes.
        view.remove_tab(2);
        assert_eq!(view.active(), Some(1));
        assert_eq!(letter(&view, 1), "c");

        // The active one: the next one takes its place.
        view.add_tab("Five", probe("e", true).0);
        view.set_active(1).unwrap();
        view.remove_tab(1);
        assert_eq!(view.active(), Some(1));
        assert_eq!(letter(&view, 1), "e");

        // The active and last one: the previous.
        assert_eq!(view.len(), 2);
        view.remove_tab(1);
        assert_eq!(view.active(), Some(0));
        assert_eq!(letter(&view, 0), "b");

        // The last tab.
        view.remove_tab(0);
        assert_eq!((view.len(), view.active()), (0, None));
        assert!(view.remove_tab(0).is_none());

        // And again.
        view.add_tab("Back", probe("z", true).0);
        assert_eq!(view.active(), Some(0));
    }

    #[test]
    fn removing_the_last_tab_selects_the_previous() {
        let mut view = view(TabPlacement::Top);
        view.set_active(2).unwrap();
        view.remove_tab(2);
        assert_eq!(view.active(), Some(1));
        view.remove_tab(1);
        assert_eq!(view.active(), Some(0));
    }

    #[test]
    fn set_active_and_set_title() {
        let mut view = view(TabPlacement::Top);
        assert!(view.set_active(1).is_ok());
        assert!(view.set_active(3).is_err());
        assert_eq!(view.active(), Some(1));
        assert!(TabView::new().set_active(0).is_err());

        view.set_title(1, "Second");
        view.set_title(7, "Nothing");
        assert_eq!(render(&mut view, (20, 1))[0], "_One__Second__Six_..");
        view.set_title(0, StyledString::plain("1"));
        assert_eq!(render(&mut view, (20, 1))[0], "_1__Second__Six_....");
    }

    #[test]
    fn emptied_view() {
        let mut view = view(TabPlacement::Top);
        while view.remove_tab(0).is_some() {}
        assert_eq!(render(&mut view, (8, 3)), ["........"; 3]);
        assert_eq!(view.required_size(Vec2::new(8, 3)), Vec2::zero());
        assert!(view.take_focus(Direction::none()).is_err());
        assert!(!send(&mut view, Key::Left));
        assert!(!send(&mut view, press((0, 0), (1, 1))));
        assert_eq!(view.active(), None);

        let mut view = TabView::default();
        assert_eq!(render(&mut view, (8, 3)), ["........"; 3]);
        assert!(view.call_on_name::<TextView, _, _>("x", |_| ()).is_none());
        assert!(view.focus_view(&Selector::Name("x")).is_err());
    }

    #[test]
    fn hidden_bar() {
        for placement in [
            TabPlacement::Top,
            TabPlacement::Bottom,
            TabPlacement::Left,
            TabPlacement::Right,
        ] {
            let mut view = view(placement).show_bar(false);
            assert_eq!(
                render(&mut view, (6, 2)),
                ["aaaaaa", "aaaaaa"],
                "{placement:?}"
            );
            assert_eq!(view.required_size(Vec2::new(10, 10)), Vec2::new(2, 2));

            // The bar cannot be focused, or used.
            view.take_focus(Direction::none()).unwrap();
            assert_eq!(view.focus_part(), Some(Part::Content));
            assert!(!send(&mut view, Key::Home));
            view.layout(Vec2::new(6, 2));
            send(&mut view, press((0, 0), (1, 0)));
            assert_eq!(view.focus_part(), Some(Part::Content));
            assert_eq!(view.active(), Some(0));
        }

        let mut view = view(TabPlacement::Top).show_bar(false);
        view.set_show_bar(true);
        assert_eq!(render(&mut view, (16, 2))[0], "_One__Two__Six_.");
    }

    #[test]
    fn single_tab_bar() {
        let mut view = TabView::new().tab("One", probe("a", true).0);
        assert_eq!(render(&mut view, (7, 2)), ["_One_..", "aaaaaaa"]);
        view.layout(Vec2::new(7, 2));
        // The bar is not focusable: the focus is for the view.
        view.take_focus(Direction::up()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert!(!send(&mut view, Key::Up));
        // Clicking the only title does nothing.
        send(&mut view, press((0, 0), (2, 0)));
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert_eq!(view.active(), Some(0));
    }

    #[test]
    fn call_on_any_reaches_every_tab() {
        let mut view = TabView::new()
            .tab("One", TextView::new("first").with_name("first"))
            .tab("Two", TextView::new("second").with_name("second"));
        assert_eq!(view.active(), Some(0));

        let found = view.call_on_name("second", |text: &mut TextView| {
            text.set_content("changed");
            text.get_content().source().to_string()
        });
        assert_eq!(found.as_deref(), Some("changed"));
        assert!(
            view.call_on_name("nothing", |_: &mut TextView| ())
                .is_none()
        );

        // Only the active tab is shown, but the other one has the new content.
        assert_eq!(view.active(), Some(0));
        let text = view.get_tab_mut(1).unwrap();
        assert!(
            text.call_on_name("second", |text: &mut TextView| {
                text.get_content().source().to_string() == "changed"
            }) == Some(true)
        );
    }

    #[test]
    fn focus_view_activates_the_tab() {
        let mut view = TabView::new()
            .tab("One", EditView::new().with_name("first"))
            .tab("Two", TextView::new("two"))
            .tab("Six", EditView::new().with_name("third"));
        view.layout(Vec2::new(12, 4));
        assert_eq!(view.active(), Some(0));

        assert!(view.focus_view(&Selector::Name("third")).is_ok());
        assert_eq!(view.active(), Some(2));
        assert_eq!(view.focus_part(), Some(Part::Content));

        assert!(view.focus_view(&Selector::Name("first")).is_ok());
        assert_eq!(view.active(), Some(0));

        assert!(view.focus_view(&Selector::Name("missing")).is_err());
        assert_eq!(view.active(), Some(0));

        // It works from the bar too.
        view.take_focus(Direction::up()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert!(view.focus_view(&Selector::Name("third")).is_ok());
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert_eq!(view.active(), Some(2));
    }

    #[test]
    fn focus_view_leaves_the_previous_view() {
        let (first, seen) = probe("a", true);
        let mut view = TabView::new()
            .tab("One", first)
            .tab("Two", EditView::new().with_name("target"));
        view.layout(Vec2::new(12, 4));
        view.take_focus(Direction::down()).unwrap();
        seen.events.lock().unwrap().clear();

        assert!(view.focus_view(&Selector::Name("target")).is_ok());
        assert!(matches!(
            seen.events.lock().unwrap()[..],
            [Event::FocusLost]
        ));
    }

    /// A tree of views, to check the sizes the parent sees.
    #[test]
    fn layout_key_follows_the_active_tab() {
        let mut small = probe("a", true).0;
        small.size = Vec2::new(3, 2);
        let mut large = probe("b", true).0;
        large.size = Vec2::new(5, 4);

        let tabs = TabView::new()
            .tab("One", small)
            .tab("Two", large)
            .with_name("tabs");
        let mut layout = LinearLayout::vertical()
            .child(tabs)
            .child(TextView::new("below"));

        let constraint = Vec2::new(40, 40);
        // The bar is 1 line, the active view 2, then the text.
        assert_eq!(layout.required_size(constraint), Vec2::new(10, 4));
        layout.layout(Vec2::new(10, 4));

        let key = layout.layout_key();
        layout
            .call_on_name("tabs", |tabs: &mut TabView| tabs.set_active(1))
            .unwrap()
            .unwrap();
        assert_ne!(layout.layout_key(), key);
        assert_eq!(layout.required_size(constraint), Vec2::new(10, 6));
        layout.layout(Vec2::new(10, 6));

        // The keys are stable when nothing changes.
        assert_eq!(layout.layout_key(), layout.layout_key());

        // The title and the placement change the size, and the key.
        let key = layout.layout_key();
        layout.call_on_name("tabs", |tabs: &mut TabView| {
            tabs.set_title(0, "A very long title");
        });
        assert_ne!(layout.layout_key(), key);
        // The titles are now wider than anything else: 19 + 5 cells.
        assert_eq!(layout.required_size(constraint), Vec2::new(24, 6));

        let key = layout.layout_key();
        layout.call_on_name("tabs", |tabs: &mut TabView| {
            tabs.set_bar_placement(TabPlacement::Left);
        });
        assert_ne!(layout.layout_key(), key);
        // A column as wide as the longest title (plus padding), next to the 5 columns
        // of the active view.
        let width = "A very long title".len() + 2 + 5;
        assert_eq!(layout.required_size(constraint), Vec2::new(width, 5));
    }

    #[test]
    fn layout_key_follows_the_active_view_and_the_bar() {
        let mut view = TabView::new()
            .tab("One", TextView::new("short"))
            .tab("Two", TextView::new("other"));
        let key = view.layout_key();
        assert_eq!(view.layout_key(), key);

        // The active view changes its size.
        view.get_tab_mut(0)
            .unwrap()
            .get_mut::<TextView>()
            .unwrap()
            .set_content("a longer text");
        let changed = view.layout_key();
        assert_ne!(changed, key);

        // The others do not matter.
        view.get_tab_mut(1)
            .unwrap()
            .get_mut::<TextView>()
            .unwrap()
            .set_content("other");
        assert_eq!(view.layout_key(), changed);

        let key = view.layout_key();
        view.set_show_bar(false);
        assert_ne!(view.layout_key(), key);
        let key = view.layout_key();
        view.set_show_bar(true);
        assert_ne!(view.layout_key(), key);

        let key = view.layout_key();
        assert!(view.remove_tab(1).is_some());
        assert_ne!(view.layout_key(), key);
        let key = view.layout_key();
        view.insert_tab(1, "New", TextView::new("new"));
        assert_ne!(view.layout_key(), key);
    }

    #[test]
    fn relayout_is_requested() {
        let mut view = view(TabPlacement::Top);
        assert!(view.needs_relayout());
        view.layout(Vec2::new(10, 5));
        assert!(!view.needs_relayout());
        view.set_active(1).unwrap();
        assert!(view.needs_relayout());
        view.layout(Vec2::new(10, 5));
        assert!(!view.needs_relayout());
        // Focus changes do not affect the layout.
        view.take_focus(Direction::down()).unwrap();
        assert!(!view.needs_relayout());
    }

    #[test]
    fn placement_from_str() {
        assert_eq!("top".parse(), Ok(TabPlacement::Top));
        assert_eq!("bottom".parse(), Ok(TabPlacement::Bottom));
        assert_eq!("left".parse(), Ok(TabPlacement::Left));
        assert_eq!("right".parse(), Ok(TabPlacement::Right));
        assert_eq!("middle".parse::<TabPlacement>(), Err(()));
        assert_eq!(TabPlacement::default(), TabPlacement::Top);
    }

    #[test]
    fn setters_and_chained_methods_agree() {
        let mut view = TabView::new();
        view.set_bar_placement(TabPlacement::Right);
        view.set_bar_align(HAlign::Center);
        view.set_show_bar(false);
        assert_eq!(view.placement, TabPlacement::Right);
        assert_eq!(view.align, HAlign::Center);
        assert!(!view.show_bar);

        let view = TabView::new()
            .bar_placement(TabPlacement::Bottom)
            .bar_align(HAlign::Right)
            .show_bar(false);
        assert_eq!(view.placement, TabPlacement::Bottom);
        assert_eq!(view.align, HAlign::Right);
        assert!(!view.show_bar);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config() {
        let config = r#"
            TabView:
                bar_placement: left
                bar_align: center
                show_bar: true
                active: 1
                tabs:
                    - title: One
                      view:
                          TextView:
                              content: first
                    - title: Two
                      view:
                          TextView:
                              content: second
        "#;
        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let tabs = view
            .downcast_ref::<TabView>()
            .expect("the view should be a TabView");
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs.active(), Some(1));
        assert_eq!(tabs.placement, TabPlacement::Left);
        assert_eq!(tabs.align, HAlign::Center);
        assert!(tabs.show_bar);
        assert_eq!(tabs.shared.lock().titles[0].source(), "One");
        assert_eq!(tabs.shared.lock().titles[1].source(), "Two");
        // The second tab is the active one.
        let mut view = crate::builder::Context::new().build(&config).unwrap();
        let tabs = view.downcast_mut::<TabView>().unwrap();
        assert_eq!(render(tabs, (12, 2))[0], "_One_second.");
        assert_eq!(render(tabs, (12, 2))[1], "_Two_.......");
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_minimal_config() {
        let config: crate::builder::Config = yaml_serde::from_str("TabView: {}").unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let tabs = view.downcast_ref::<TabView>().unwrap();
        assert!(tabs.is_empty());
        assert_eq!(tabs.placement, TabPlacement::Top);
        assert!(tabs.show_bar);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_invalid_config() {
        let bad = [
            // Out of range.
            r#"{"TabView": {"active": 2, "tabs": [{"title": "A", "view": {"TextView": {"content": "a"}}}]}}"#,
            r#"{"TabView": {"bar_placement": "middle"}}"#,
            r#"{"TabView": {"bar_align": "up"}}"#,
            r#"{"TabView": {"show_bar": "maybe"}}"#,
            // No view, no title, not an object.
            r#"{"TabView": {"tabs": [{"title": "A"}]}}"#,
            r#"{"TabView": {"tabs": [{"view": {"TextView": {"content": "a"}}}]}}"#,
            r#"{"TabView": {"tabs": ["A"]}}"#,
        ];
        for config in bad {
            let config: crate::builder::Config =
                crate::reexports::serde_json::from_str(config).unwrap();
            assert!(
                crate::builder::Context::new().build(&config).is_err(),
                "{config}"
            );
        }
    }

    // ----- Detached bars -----

    /// A view with three tabs "One", "Two" and "Six", and a detached bar.
    fn detached() -> (TabView, TabBar) {
        let mut view = view(TabPlacement::Top);
        let bar = view.detached_bar();
        (view, bar)
    }

    fn send_to<V: View>(view: &mut V, event: impl Into<Event>) -> bool {
        let result = view.on_event(event.into());
        let consumed = result.is_consumed();
        result.process(&mut Cursive::new());
        consumed
    }

    fn bar_buffer(
        bar: &mut TabBar,
        size: impl Into<Vec2>,
        focused: bool,
        theme: &Theme,
    ) -> RwLock<PrintBuffer> {
        let size = size.into();
        bar.layout(size);
        let mut buffer = PrintBuffer::new();
        buffer.resize(size + (3, 2));
        let buffer = RwLock::new(buffer);
        bar.draw(&Printer::new(size, theme, &buffer).focused(focused));
        buffer
    }

    fn text_of(buffer: &RwLock<PrintBuffer>, size: Vec2) -> Vec<String> {
        let buffer = buffer.read();
        (0..size.y)
            .map(|y| {
                (0..size.x)
                    .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                        None | Some("") => ".",
                        Some(" ") => "_",
                        Some(text) => text,
                    })
                    .collect::<String>()
            })
            .collect()
    }

    fn render_bar(bar: &mut TabBar, size: impl Into<Vec2>) -> Vec<String> {
        let size = size.into();
        let buffer = bar_buffer(bar, size, true, &Theme::retro());
        text_of(&buffer, size)
    }

    #[test]
    fn detached_bar_hides_the_builtin_bar() {
        let mut view = view(TabPlacement::Top);
        assert_eq!(render(&mut view, (16, 2))[0], "_One__Two__Six_.");
        let _bar = view.detached_bar();
        assert_eq!(render(&mut view, (16, 2)), ["aaaaaaaaaaaaaaaa"; 2]);
        assert!(!view.show_bar);
        assert_eq!(view.required_size(Vec2::new(10, 10)), Vec2::new(2, 2));

        // Both are shown if the bar is shown again.
        view.set_show_bar(true);
        assert_eq!(render(&mut view, (16, 2))[0], "_One__Two__Six_.");
    }

    #[test]
    fn detached_vertical_bar() {
        let (_view, mut bar) = detached();
        assert_eq!(
            render_bar(&mut bar, (7, 4)),
            ["_One___", "_Two___", "_Six___", "......."]
        );
        // Vertical by default: a column as wide as the widest title.
        assert_eq!(bar.required_size(Vec2::new(40, 40)), Vec2::new(5, 3));
        // It does not depend on the space offered.
        assert_eq!(bar.required_size(Vec2::new(4, 2)), Vec2::new(5, 3));
    }

    #[test]
    fn detached_horizontal_bar() {
        let (mut view, bar) = detached();
        let mut bar = bar.orientation(Orientation::Horizontal);
        assert_eq!(
            render_bar(&mut bar, (16, 2)),
            ["_One__Two__Six_.", "................"]
        );
        assert_eq!(bar.required_size(Vec2::new(40, 40)), Vec2::new(15, 1));
        assert_eq!(bar.required_size(Vec2::new(10, 40)), Vec2::new(15, 1));

        bar.set_align(HAlign::Right);
        assert_eq!(render_bar(&mut bar, (18, 1))[0], "..._One__Two__Six_");
        let bar = bar.align(HAlign::Center);
        let mut bar = bar;
        assert_eq!(render_bar(&mut bar, (19, 1))[0], ".._One__Two__Six_..");

        // Scrolls like the built-in bar.
        view.set_active(2).unwrap();
        assert_eq!(render_bar(&mut bar, (6, 1))[0], "_Six_.");
    }

    #[test]
    fn detached_bar_styles() {
        let theme = Theme::retro();
        let highlight = palette_style(PaletteStyle::Highlight, &theme);
        let inactive = palette_style(PaletteStyle::HighlightInactive, &theme);
        let plain = {
            let mut buffer = PrintBuffer::new();
            buffer.resize(Vec2::new(1, 1));
            let buffer = RwLock::new(buffer);
            Printer::new((1, 1), &theme, &buffer).print((0, 0), " ");
            cell_style(&buffer, 0, 0)
        };
        let (mut view, bar) = detached();
        view.set_active(1).unwrap();

        // Vertical: the whole width of the active line.
        let mut vertical = bar;
        let focused = bar_buffer(&mut vertical, (8, 3), true, &theme);
        for x in 0..8 {
            assert_eq!(cell_style(&focused, x, 1), highlight, "{x}");
            assert_eq!(cell_style(&focused, x, 0), plain);
            assert_eq!(cell_style(&focused, x, 2), plain);
        }
        let unfocused = bar_buffer(&mut vertical, (8, 3), false, &theme);
        assert_eq!(cell_style(&unfocused, 3, 1), inactive);
        assert_eq!(cell_style(&unfocused, 3, 0), plain);

        // Horizontal: the padded title.
        let mut horizontal = view.detached_bar().orientation(Orientation::Horizontal);
        let focused = bar_buffer(&mut horizontal, (16, 1), true, &theme);
        for x in 0..15 {
            let expected = if (5..10).contains(&x) {
                highlight
            } else {
                plain
            };
            assert_eq!(cell_style(&focused, x, 0), expected, "{x}");
        }
        let unfocused = bar_buffer(&mut horizontal, (16, 1), false, &theme);
        assert_eq!(cell_style(&unfocused, 6, 0), inactive);
    }

    #[test]
    fn vertical_bar_keys() {
        let (mut view, mut bar) = detached();
        bar.layout(Vec2::new(5, 3));
        assert!(bar.take_focus(Direction::none()).is_ok());

        // The start: nothing before, for the parent to handle.
        assert!(!send_to(&mut bar, Key::Up));
        assert!(send_to(&mut bar, Key::Down));
        assert_eq!(view.active(), Some(1));
        assert!(send_to(&mut bar, Key::Down));
        assert_eq!(view.active(), Some(2));
        // The end: no wrap-around.
        assert!(!send_to(&mut bar, Key::Down));
        assert_eq!(view.active(), Some(2));
        assert!(send_to(&mut bar, Key::Up));
        assert_eq!(view.active(), Some(1));

        assert!(send_to(&mut bar, Key::Home));
        assert_eq!(view.active(), Some(0));
        assert!(send_to(&mut bar, Key::End));
        assert_eq!(view.active(), Some(2));
        // Already there: ignored, like an arrow at the end.
        assert!(!send_to(&mut bar, Key::End));

        // Not the keys of a vertical bar.
        assert!(!send_to(&mut bar, Key::Left));
        assert!(!send_to(&mut bar, Key::Right));
        assert!(!send_to(&mut bar, Key::Tab));
        assert!(!send_to(&mut bar, Event::Char('x')));
        assert_eq!(view.active(), Some(2));

        // What the view draws follows.
        assert_eq!(render(&mut view, (4, 1)), ["cccc"]);
    }

    #[test]
    fn horizontal_bar_keys() {
        let (view, bar) = detached();
        let mut bar = bar.orientation(Orientation::Horizontal);
        bar.layout(Vec2::new(15, 1));
        assert!(!send_to(&mut bar, Key::Left));
        assert!(send_to(&mut bar, Key::Right));
        assert!(send_to(&mut bar, Key::Right));
        assert!(!send_to(&mut bar, Key::Right));
        assert_eq!(view.active(), Some(2));
        assert!(send_to(&mut bar, Key::Left));
        assert_eq!(view.active(), Some(1));
        assert!(!send_to(&mut bar, Key::Up));
        assert!(!send_to(&mut bar, Key::Down));
        assert_eq!(view.active(), Some(1));
    }

    #[test]
    fn bar_is_focusable_with_several_tabs() {
        let mut view = TabView::new().tab("One", probe("a", true).0);
        let mut bar = view.detached_bar();
        assert!(bar.take_focus(Direction::none()).is_err());
        view.add_tab("Two", probe("b", true).0);
        assert!(bar.take_focus(Direction::none()).is_ok());
        view.remove_tab(0);
        assert!(bar.take_focus(Direction::none()).is_err());

        let mut empty = TabView::new();
        let mut bar = empty.detached_bar();
        assert!(bar.take_focus(Direction::none()).is_err());
        assert!(!send_to(&mut bar, Key::Down));
        assert_eq!(render_bar(&mut bar, (4, 2)), ["....", "...."]);
        assert_eq!(bar.required_size(Vec2::new(9, 9)), Vec2::zero());
    }

    #[test]
    fn mouse_on_a_detached_bar() {
        let (view, mut bar) = detached();
        bar.layout(Vec2::new(5, 4));
        // The bar is at (3, 4).
        assert!(send_to(&mut bar, press((3, 4), (5, 6))));
        assert_eq!(view.active(), Some(2));
        assert!(send_to(&mut bar, press((3, 4), (3, 4))));
        assert_eq!(view.active(), Some(0));
        // Under the last title, still in the bar: nothing to select.
        assert!(send_to(&mut bar, press((3, 4), (4, 7))));
        assert_eq!(view.active(), Some(0));
        // Outside, other buttons, other events.
        assert!(!send_to(&mut bar, press((3, 4), (4, 8))));
        assert!(!send_to(&mut bar, press((5, 5), (2, 2))));
        for event in [MouseEvent::WheelDown, MouseEvent::Press(MouseButton::Right)] {
            let event = Event::Mouse {
                offset: Vec2::zero(),
                position: Vec2::new(1, 2),
                event,
            };
            assert!(!send_to(&mut bar, event));
        }
        assert_eq!(view.active(), Some(0));

        // Horizontal.
        let mut bar = view_bar_horizontal(&view);
        bar.layout(Vec2::new(15, 1));
        assert!(send_to(&mut bar, press((0, 0), (6, 0))));
        assert_eq!(view.active(), Some(1));
        assert!(send_to(&mut bar, press((0, 0), (12, 0))));
        assert_eq!(view.active(), Some(2));
    }

    // Another bar for the same state (a `TabView` is only needed to get one).
    fn view_bar_horizontal(view: &TabView) -> TabBar {
        TabBar::with_shared(Arc::clone(&view.shared)).orientation(Orientation::Horizontal)
    }

    #[test]
    fn tabview_changes_are_reflected_by_the_bar() {
        let (mut view, mut bar) = detached();
        let size = (7, 4);
        let rows = |bar: &mut TabBar| render_bar(bar, size);

        let theme = Theme::retro();
        let highlight = palette_style(PaletteStyle::Highlight, &theme);
        let active_row = |bar: &mut TabBar| {
            let buffer = bar_buffer(bar, size, true, &theme);
            (0..4)
                .find(|&y| cell_style(&buffer, 0, y) == highlight)
                .unwrap()
        };

        assert_eq!(active_row(&mut bar), 0);
        view.set_active(2).unwrap();
        assert_eq!(active_row(&mut bar), 2);

        view.insert_tab(0, "Zero", probe("z", true).0);
        assert_eq!(rows(&mut bar), ["_Zero__", "_One___", "_Two___", "_Six___"]);
        // Still the same tab, one line lower.
        assert_eq!(active_row(&mut bar), 3);

        view.add_tab("End", probe("e", true).0);
        assert_eq!(view.len(), 5);
        view.set_title(1, "First");
        assert_eq!(rows(&mut bar)[1], "_First_");

        view.remove_tab(0);
        assert_eq!(rows(&mut bar)[0], "_First_");
        assert_eq!(active_row(&mut bar), 2);

        view.remove_tab(2);
        view.remove_tab(2);
        // The active one was removed: the previous one is selected.
        assert_eq!(view.active(), Some(1));
        assert_eq!(active_row(&mut bar), 1);
        view.remove_tab(0);
        view.remove_tab(0);
        assert_eq!(rows(&mut bar), ["......."; 4]);
        assert!(view.is_empty());
    }

    #[test]
    fn two_bars_stay_in_sync() {
        let mut view = view(TabPlacement::Top);
        let mut first = view.detached_bar();
        let mut second = view.detached_bar().orientation(Orientation::Horizontal);
        first.layout(Vec2::new(5, 3));
        second.layout(Vec2::new(15, 1));

        assert!(send_to(&mut first, Key::Down));
        assert_eq!(view.active(), Some(1));
        // The other bar follows: it moves from the second title.
        assert!(send_to(&mut second, Key::Right));
        assert_eq!(view.active(), Some(2));
        assert!(!send_to(&mut first, Key::Down));
        assert!(send_to(&mut second, Key::Home));
        assert_eq!(view.active(), Some(0));

        view.set_title(0, "Uno");
        assert_eq!(render_bar(&mut first, (7, 3))[0], "_Uno___");
        assert_eq!(render_bar(&mut second, (16, 1))[0], "_Uno__Two__Six_.");
    }

    #[test]
    fn both_on_change_callbacks_run_once_per_user_change() {
        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let view_log = Arc::clone(&log);
        let mut view = view(TabPlacement::Top).on_change(move |_, index| {
            view_log.lock().unwrap().push(format!("view {index}"));
        });
        let bar_log = Arc::clone(&log);
        let mut bar = view.detached_bar().on_change(move |_, index| {
            bar_log.lock().unwrap().push(format!("bar {index}"));
        });
        let other_log = Arc::clone(&log);
        let mut other = view.detached_bar();
        other.set_on_change(move |_, index| {
            other_log.lock().unwrap().push(format!("other {index}"));
        });
        bar.layout(Vec2::new(5, 3));
        other.layout(Vec2::new(5, 3));

        // Programmatic changes: no call.
        view.set_active(2).unwrap();
        view.remove_tab(0);
        view.insert_tab(0, "New", probe("n", true).0);
        view.set_title(0, "Newer");
        assert!(log.lock().unwrap().is_empty());

        // The view first, then the bar that was used.
        assert!(send_to(&mut bar, Key::Up));
        assert_eq!(*log.lock().unwrap(), ["view 1", "bar 1"]);
        log.lock().unwrap().clear();

        assert!(send_to(&mut other, Key::Home));
        assert_eq!(*log.lock().unwrap(), ["view 0", "other 0"]);
        log.lock().unwrap().clear();

        // No change, no call (also at the ends).
        assert!(!send_to(&mut bar, Key::Home));
        assert!(!send_to(&mut bar, Key::Up));
        assert!(log.lock().unwrap().is_empty());

        // The mouse too.
        assert!(send_to(&mut bar, press((0, 0), (1, 2))));
        assert_eq!(*log.lock().unwrap(), ["view 2", "bar 2"]);
        log.lock().unwrap().clear();

        // Without a callback on the view, only the bar's.
        let mut view = TabView::new()
            .tab("One", probe("a", true).0)
            .tab("Two", probe("b", true).0);
        let seen = Arc::clone(&log);
        let mut bar = view
            .detached_bar()
            .on_change(move |_, index| seen.lock().unwrap().push(format!("bar {index}")));
        assert!(send_to(&mut bar, Key::End));
        assert_eq!(*log.lock().unwrap(), ["bar 1"]);

        // And the callback of the view alone, with a bar that has none.
        let (mut view, changes) = recording(self::view(TabPlacement::Top));
        let mut bar = view.detached_bar();
        assert!(send_to(&mut bar, Key::End));
        assert_eq!(*changes.lock().unwrap(), [2]);
    }

    #[test]
    fn the_builtin_bar_still_calls_the_view_callback_with_a_detached_one() {
        let (mut view, changes) = recording(view(TabPlacement::Top));
        let mut bar = view.detached_bar();
        view.set_show_bar(true);
        view.take_focus(Direction::none()).unwrap();
        assert!(send(&mut view, Key::Right));
        assert!(send_to(&mut bar, Key::End));
        assert_eq!(*changes.lock().unwrap(), [1, 2]);
        assert_eq!(view.active(), Some(2));
    }

    fn keys_of(layout: &mut LinearLayout) -> (u64, u64) {
        let bar = layout.call_on_name("bar", |bar: &mut TabBar| bar.layout_key());
        let tabs = layout.call_on_name("tabs", |tabs: &mut TabView| tabs.layout_key());
        (bar.unwrap(), tabs.unwrap())
    }

    #[test]
    fn layout_keys_follow_the_bar() {
        let mut small = probe("a", true).0;
        small.size = Vec2::new(3, 2);
        let mut large = probe("b", true).0;
        large.size = Vec2::new(5, 4);
        let mut tabs = TabView::new().tab("One", small).tab("Two", large);
        let bar = tabs.detached_bar();

        let mut layout = LinearLayout::horizontal()
            .child(bar.with_name("bar"))
            .child(tabs.with_name("tabs"));
        let constraint = Vec2::new(40, 40);
        // The column is 5 wide, the active view 3x2.
        assert_eq!(layout.required_size(constraint), Vec2::new(8, 2));
        layout.layout(Vec2::new(8, 2));
        assert!(!layout.needs_relayout());

        let key = layout.layout_key();
        let (bar_key, tabs_key) = keys_of(&mut layout);

        // The user changes the tab from the bar.
        layout
            .call_on_name("bar", |bar: &mut TabBar| send_to(bar, Key::Down))
            .unwrap();
        assert!(layout.needs_relayout());
        assert_ne!(layout.layout_key(), key);
        let (new_bar_key, new_tabs_key) = keys_of(&mut layout);
        // The content may have another size: its key changes. The titles did not.
        assert_ne!(new_tabs_key, tabs_key);
        assert_eq!(new_bar_key, bar_key);
        assert_eq!(layout.required_size(constraint), Vec2::new(10, 4));
        layout.layout(Vec2::new(10, 4));
        assert!(!layout.needs_relayout());
        // Stable when nothing changes.
        assert_eq!(layout.layout_key(), layout.layout_key());
        assert_eq!(keys_of(&mut layout), (new_bar_key, new_tabs_key));

        // Titles changed in the view: both keys change, both sizes follow.
        layout.call_on_name("tabs", |tabs: &mut TabView| {
            tabs.set_title(0, "A very long title");
        });
        assert!(layout.needs_relayout());
        let (bar_key, tabs_key) = keys_of(&mut layout);
        assert_ne!(bar_key, new_bar_key);
        assert_ne!(tabs_key, new_tabs_key);
        // 17 characters, plus padding, and the 5 columns of the active view.
        assert_eq!(layout.required_size(constraint), Vec2::new(19 + 5, 4));
        layout.layout(Vec2::new(24, 4));
        assert!(!layout.needs_relayout());

        // Adding and removing tabs, too.
        layout.call_on_name("tabs", |tabs: &mut TabView| {
            tabs.add_tab("x", probe("x", true).0);
        });
        assert!(layout.needs_relayout());
        assert_ne!(keys_of(&mut layout).0, bar_key);
        layout.layout(Vec2::new(24, 4));
        let bar_key = keys_of(&mut layout).0;
        layout.call_on_name("tabs", |tabs: &mut TabView| tabs.remove_tab(2));
        assert_ne!(keys_of(&mut layout).0, bar_key);

        // The orientation of a bar changes its size.
        layout.layout(Vec2::new(24, 4));
        let key = layout.layout_key();
        layout.call_on_name("bar", |bar: &mut TabBar| {
            bar.set_orientation(Orientation::Horizontal)
        });
        assert_ne!(layout.layout_key(), key);
        // The titles side by side: 19 + 5, and the 5 columns of the view.
        assert_eq!(layout.required_size(constraint), Vec2::new(19 + 5 + 5, 4));
    }

    #[test]
    fn bar_key_changes_with_titles_only() {
        let (mut view, mut bar) = detached();
        bar.layout(Vec2::new(5, 3));
        let key = bar.layout_key();
        assert!(!bar.needs_relayout());

        // The active tab does not matter to the bar's size.
        view.set_active(1).unwrap();
        assert_eq!(bar.layout_key(), key);
        assert!(!bar.needs_relayout());

        view.set_title(0, "Another");
        assert_ne!(bar.layout_key(), key);
        assert!(bar.needs_relayout());
        bar.layout(Vec2::new(9, 3));
        assert!(!bar.needs_relayout());
    }

    #[test]
    fn content_keeps_the_focus_when_a_bar_changes_the_tab() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", true);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        let mut bar = view.detached_bar();
        view.layout(Vec2::new(6, 3));
        bar.layout(Vec2::new(5, 2));
        view.take_focus(Direction::none()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Content));
        seen_first.events.lock().unwrap().clear();

        // The tab is changed from the bar.
        assert!(send_to(&mut bar, Key::Down));
        assert_eq!(view.active(), Some(1));
        // The view notices when it is laid out: the old view is told...
        view.layout(Vec2::new(6, 3));
        assert!(matches!(
            seen_first.events.lock().unwrap()[..],
            [Event::FocusLost]
        ));
        // ... and the new one gets the focus the content had.
        assert_eq!(*seen_second.sources.lock().unwrap(), [Direction::none()]);
        assert!(seen_second.events.lock().unwrap().is_empty());
        assert!(view.content_focused());

        // If the parent then moves the focus away from the view, the new view is the one told.
        send(&mut view, Event::FocusLost);
        assert!(matches!(
            seen_second.events.lock().unwrap()[..],
            [Event::FocusLost]
        ));
        assert_eq!(seen_first.events.lock().unwrap().len(), 1);
    }

    #[test]
    fn content_loses_the_focus_when_the_new_tab_refuses_it() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", false);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        let mut bar = view.detached_bar();
        view.layout(Vec2::new(6, 3));
        bar.layout(Vec2::new(5, 2));
        view.take_focus(Direction::none()).unwrap();
        seen_first.events.lock().unwrap().clear();

        assert!(send_to(&mut bar, Key::Down));
        view.layout(Vec2::new(6, 3));
        assert!(matches!(
            seen_first.events.lock().unwrap()[..],
            [Event::FocusLost]
        ));
        // Nothing has the focus in the content any more.
        assert!(view.focus_part().is_none());
        assert!(!view.content_focused());

        // The parent moving the focus away tells nobody: the new view never had it.
        assert!(!send(&mut view, Event::FocusLost));
        assert!(seen_second.events.lock().unwrap().is_empty());
        assert_eq!(seen_first.events.lock().unwrap().len(), 1);
    }

    #[test]
    fn content_is_told_when_the_event_comes_first() {
        // No layout in between: the next event to the view does it.
        let (first, seen_first) = probe("a", true);
        let mut view = TabView::new()
            .tab("One", first)
            .tab("Two", probe("b", true).0);
        let mut bar = view.detached_bar();
        view.take_focus(Direction::none()).unwrap();
        seen_first.events.lock().unwrap().clear();
        assert!(send_to(&mut bar, Key::Down));
        // The event for the parent's focus change goes to nobody that had the focus.
        send(&mut view, Event::FocusLost);
        assert!(matches!(
            seen_first.events.lock().unwrap()[..],
            [Event::FocusLost]
        ));
        // Events go to the active tab, not to the previous one.
        assert_eq!(view.active(), Some(1));
        send(&mut view, Event::Char('x'));
        assert_eq!(seen_first.events.lock().unwrap().len(), 1);
    }

    #[test]
    fn no_focus_lost_when_the_content_had_no_focus() {
        let (first, seen_first) = probe("a", true);
        let mut view = TabView::new()
            .tab("One", first)
            .tab("Two", probe("b", true).0);
        let mut bar = view.detached_bar();
        view.layout(Vec2::new(6, 3));
        // Never focused.
        assert!(send_to(&mut bar, Key::Down));
        view.layout(Vec2::new(6, 3));
        assert!(seen_first.events.lock().unwrap().is_empty());
        assert!(view.focus_part().is_some());
    }

    #[test]
    fn programmatic_changes_move_the_focus_with_the_active_view() {
        // Detached bar: no bar to fall back to.
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", true);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        let mut bar = view.detached_bar();
        view.take_focus(Direction::none()).unwrap();
        seen_first.events.lock().unwrap().clear();
        view.set_active(1).unwrap();
        view.layout(Vec2::new(6, 3));
        // The old view is told, once, and the new one is given the focus.
        assert!(matches!(
            seen_first.events.lock().unwrap()[..],
            [Event::FocusLost]
        ));
        assert_eq!(*seen_second.sources.lock().unwrap(), [Direction::none()]);
        assert_eq!(view.focus_part(), Some(Part::Content));
        assert!(view.content_focused());
        send(&mut view, Event::Char('x'));
        assert!(matches!(
            seen_second.events.lock().unwrap()[..],
            [Event::Char('x')]
        ));
        assert_eq!(seen_first.events.lock().unwrap().len(), 1);
        // The bar does not need to be told.
        assert!(send_to(&mut bar, Key::Up));
        assert_eq!(view.active(), Some(0));
    }

    #[test]
    fn set_active_to_a_view_that_refuses_the_focus() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", false);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        view.take_focus(Direction::back()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Content));
        seen_first.events.lock().unwrap().clear();
        view.set_active(1).unwrap();
        assert!(matches!(
            seen_first.events.lock().unwrap()[..],
            [Event::FocusLost]
        ));
        // The focus is on the bar, and the view that refused gets no key.
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert!(!view.content_focused());
        send(&mut view, Event::Char('x'));
        assert!(seen_second.events.lock().unwrap().is_empty());
        // The bar works.
        assert!(send(&mut view, Key::Left));
        assert_eq!(view.active(), Some(0));
        assert!(seen_second.events.lock().unwrap().is_empty());
    }

    #[test]
    fn set_active_to_a_view_that_refuses_the_focus_without_a_bar() {
        let (first, _) = probe("a", true);
        let (second, seen_second) = probe("b", false);
        let mut view = TabView::new()
            .tab("One", first)
            .tab("Two", second)
            .show_bar(false);
        view.take_focus(Direction::none()).unwrap();
        view.set_active(1).unwrap();
        assert!(!view.content_focused());
        assert!(!send(&mut view, Event::Char('x')));
        assert!(!send(&mut view, Key::Tab));
        assert!(seen_second.events.lock().unwrap().is_empty());
        // A parent can still give the focus again.
        assert!(view.take_focus(Direction::none()).is_err());
        view.set_active(0).unwrap();
        assert!(view.take_focus(Direction::none()).is_ok());
        assert!(view.content_focused());
    }

    #[test]
    fn set_active_leaves_a_focused_bar_alone() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", true);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        view.take_focus(Direction::front()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Bar));
        view.set_active(1).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert!(view.focus_part().is_some());
        assert!(seen_first.events.lock().unwrap().is_empty());
        assert!(seen_second.events.lock().unwrap().is_empty());
        assert!(seen_second.sources.lock().unwrap().is_empty());
    }

    #[test]
    fn remove_tab_gives_the_focus_to_the_next_view() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", true);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        view.take_focus(Direction::back()).unwrap();
        seen_first.events.lock().unwrap().clear();
        let removed = view.remove_tab(0).unwrap();
        // The removed view is handed back as is.
        assert!(removed.is::<Probe>());
        assert!(seen_first.events.lock().unwrap().is_empty());
        assert_eq!(*seen_second.sources.lock().unwrap(), [Direction::none()]);
        assert!(view.content_focused());
        send(&mut view, Event::Char('x'));
        assert!(matches!(
            seen_second.events.lock().unwrap()[..],
            [Event::Char('x')]
        ));
    }

    #[test]
    fn remove_tab_to_a_view_that_refuses_the_focus() {
        let (first, _) = probe("a", true);
        let (second, seen_second) = probe("b", false);
        let mut view = TabView::new()
            .tab("One", first)
            .tab("Two", second)
            .tab("Six", probe("c", true).0);
        view.take_focus(Direction::back()).unwrap();
        view.remove_tab(0).unwrap();
        assert_eq!(view.active(), Some(0));
        assert_eq!(view.focus_part(), Some(Part::Bar));
        assert!(!view.content_focused());
        send(&mut view, Event::Char('x'));
        assert!(seen_second.events.lock().unwrap().is_empty());
        // Removing another tab than the active one changes nothing.
        view.take_focus(Direction::back()).ok();
        view.remove_tab(1).unwrap();
    }

    #[test]
    fn remove_tab_keeps_the_focus_when_another_tab_goes() {
        let (first, seen_first) = probe("a", true);
        let mut view = TabView::new()
            .tab("One", first)
            .tab("Two", probe("b", true).0);
        view.take_focus(Direction::back()).unwrap();
        seen_first.events.lock().unwrap().clear();
        view.remove_tab(1).unwrap();
        assert!(seen_first.events.lock().unwrap().is_empty());
        assert_eq!(seen_first.sources.lock().unwrap().len(), 1);
        assert!(view.content_focused());
    }

    #[test]
    fn focus_callbacks_of_programmatic_changes_run_with_the_next_event() {
        struct Notify {
            lost: Arc<AtomicUsize>,
            taken: Arc<AtomicUsize>,
        }
        impl View for Notify {
            fn draw(&self, _: &Printer) {}
            fn required_size(&mut self, _: Vec2) -> Vec2 {
                Vec2::new(1, 1)
            }
            fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
                let taken = Arc::clone(&self.taken);
                Ok(EventResult::with_cb(move |_| {
                    taken.fetch_add(1, Ordering::Relaxed);
                }))
            }
            fn on_event(&mut self, event: Event) -> EventResult {
                if event == Event::FocusLost {
                    let lost = Arc::clone(&self.lost);
                    return EventResult::with_cb(move |_| {
                        lost.fetch_add(1, Ordering::Relaxed);
                    });
                }
                EventResult::Ignored
            }
        }
        let (lost, taken) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let notify = |lost: &Arc<AtomicUsize>, taken: &Arc<AtomicUsize>| Notify {
            lost: Arc::clone(lost),
            taken: Arc::clone(taken),
        };
        let mut view = TabView::new()
            .tab("One", notify(&lost, &taken))
            .tab("Two", notify(&lost, &taken))
            .tab("Six", notify(&lost, &taken));
        view.take_focus(Direction::back()).unwrap();
        view.set_active(1).unwrap();
        view.remove_tab(1).unwrap();
        assert_eq!(lost.load(Ordering::Relaxed), 0);
        // They go with an event that is consumed. An ignored one does not carry them, but it
        // does not make them wait forever either.
        assert!(!send(&mut view, Event::Char('x')));
        assert_eq!(lost.load(Ordering::Relaxed), 0);
        assert!(send(&mut view, Event::Char('x')));
        // One for the change, none for the removal (the removed view is not told).
        assert_eq!(lost.load(Ordering::Relaxed), 1);
        // One for the first take_focus is not run (returned to the test), so: the two offers.
        assert_eq!(taken.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn a_bar_can_outlive_its_view() {
        let (view, mut bar) = detached();
        bar.layout(Vec2::new(7, 3));
        let (view, changes) = recording(view);
        drop(view);
        let _ = changes;
        // The titles it last knew.
        assert_eq!(
            render_bar(&mut bar, (7, 3)),
            ["_One___", "_Two___", "_Six___"]
        );
        // Nothing to control any more.
        assert!(bar.take_focus(Direction::none()).is_err());
        assert!(!send_to(&mut bar, Key::Down));
        assert!(!send_to(&mut bar, Key::End));
        assert!(!send_to(&mut bar, press((0, 0), (1, 1))));
        assert_eq!(render_bar(&mut bar, (7, 3))[0], "_One___");
        assert_eq!(bar.required_size(Vec2::new(9, 9)), Vec2::new(5, 3));
        assert!(!bar.needs_relayout());
    }

    #[test]
    fn dropping_the_view_releases_its_callback() {
        let token = Arc::new(());
        let held = Arc::clone(&token);
        let mut view = view(TabPlacement::Top).on_change(move |_, _| {
            let _ = &held;
        });
        let bar = view.detached_bar();
        assert_eq!(Arc::strong_count(&token), 2);
        drop(view);
        assert_eq!(Arc::strong_count(&token), 1);
        drop(bar);
    }

    #[test]
    fn a_view_whose_bars_were_dropped_is_a_view_with_a_hidden_bar() {
        let mut view = view(TabPlacement::Top);
        drop(view.detached_bar());
        assert_eq!(render(&mut view, (6, 2)), ["aaaaaa", "aaaaaa"]);
        view.take_focus(Direction::none()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Content));
        view.set_active(1).unwrap();
        assert_eq!(render(&mut view, (6, 2)), ["bbbbbb", "bbbbbb"]);
    }

    #[test]
    fn bar_setters_and_chained_methods_agree() {
        let (_view, mut bar) = detached();
        assert_eq!(bar.orientation, Orientation::Vertical);
        bar.set_orientation(Orientation::Horizontal);
        bar.set_align(HAlign::Center);
        assert_eq!(bar.orientation, Orientation::Horizontal);
        assert_eq!(bar.align, HAlign::Center);

        let (_view, bar) = detached();
        let bar = bar
            .orientation(Orientation::Horizontal)
            .align(HAlign::Right);
        assert_eq!(bar.orientation, Orientation::Horizontal);
        assert_eq!(bar.align, HAlign::Right);
    }

    #[test]
    fn bar_keeps_its_size_next_to_a_full_height_view() {
        use crate::view::Resizable;

        // A parent that shares the space does not squeeze the bar.
        let (_view, bar) = detached();
        let mut layout = LinearLayout::vertical()
            .child(bar.with_name("bar"))
            .child(TextView::new("filler").full_height());
        assert_eq!(layout.required_size(Vec2::new(10, 20)), Vec2::new(6, 20));
        layout.layout(Vec2::new(10, 20));
        let size = layout
            .call_on_name("bar", |bar: &mut TabBar| bar.last_size)
            .unwrap();
        assert_eq!(size.y, 3);
    }

    // ----- Focus, mouse and lifetime -----
    fn lost_count(seen: &Seen) -> usize {
        seen.events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| matches!(event, Event::FocusLost))
            .count()
    }

    /// The parent tells the view it lost the focus (a sibling took it), then the detached bar
    /// changes the tab. The old view was already told.
    #[test]
    fn focus_lost_once_when_the_parent_took_the_focus_before_a_bar_switch() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", true);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        let mut bar = view.detached_bar();
        view.layout(Vec2::new(6, 3));
        bar.layout(Vec2::new(5, 2));
        view.take_focus(Direction::none()).unwrap();
        // Like LinearLayout does when the focus goes to the sibling holding the bar.
        send(&mut view, Event::FocusLost);
        assert_eq!(lost_count(&seen_first), 1);

        assert!(send_to(&mut bar, Key::Down));
        view.layout(Vec2::new(6, 3));
        assert_eq!(lost_count(&seen_first), 1, "FocusLost sent twice");
        assert!(
            seen_second.sources.lock().unwrap().is_empty(),
            "the new view was offered the focus although the TabView does not have it"
        );
    }

    /// Same with `set_active`.
    #[test]
    fn set_active_after_the_parent_took_the_focus() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", true);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        view.take_focus(Direction::back()).unwrap();
        assert!(view.content_focused());
        send(&mut view, Event::FocusLost);
        view.set_active(1).unwrap();
        assert_eq!(lost_count(&seen_first), 1, "FocusLost sent twice");
        assert!(seen_second.sources.lock().unwrap().is_empty());
    }

    /// `focus_view` to another tab when the active view already refused the focus.
    #[test]
    fn focus_view_does_not_tell_a_view_that_refused_the_focus() {
        let (first, seen_first) = probe("a", true);
        let (second, _) = probe("b", false);
        let mut view = TabView::new()
            .tab("One", first)
            .tab("Two", second)
            .tab("Three", EditView::new().with_name("target2"))
            .show_bar(false);
        view.take_focus(Direction::none()).unwrap();
        view.set_active(1).unwrap();
        assert_eq!(lost_count(&seen_first), 1);
        // Nothing has the focus. Now go to a view in the last tab.
        view.focus_view(&Selector::Name("target2")).unwrap();
        assert_eq!(lost_count(&seen_first), 1);
        // The refusing view in between was never given the focus, so it must not be told.
        let seen_second = view
            .get_tab(1)
            .unwrap()
            .get::<Probe>()
            .unwrap()
            .seen
            .clone();
        assert_eq!(
            lost_count(&seen_second),
            0,
            "FocusLost to a view that never had the focus"
        );
    }

    /// An ignored key is reported as consumed when a callback was pending.
    #[test]
    fn pending_callback_does_not_swallow_an_unused_key() {
        struct Cb;
        impl View for Cb {
            fn draw(&self, _: &Printer) {}
            fn required_size(&mut self, _: Vec2) -> Vec2 {
                Vec2::new(1, 1)
            }
            fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
                Ok(EventResult::consumed())
            }
            fn on_event(&mut self, event: Event) -> EventResult {
                match event {
                    Event::FocusLost => EventResult::with_cb(|_| ()),
                    _ => EventResult::Ignored,
                }
            }
        }
        let mut view = TabView::new().tab("One", Cb).tab("Two", probe("b", true).0);
        view.layout(Vec2::new(6, 3));
        view.take_focus(Direction::back()).unwrap();
        assert!(view.content_focused());
        view.set_active(1).unwrap();
        // Content (probe b) has the focus and ignores keys; Left/Right/Up/Down do nothing in a
        // top bar except Up (goes to the bar). A plain 'x' is unused by everyone:
        assert!(
            !send(&mut view, Event::Char('x')),
            "unused key swallowed by the pending result"
        );
    }

    /// The bar had the focus, and the number of tabs drops to one: the view that never took the
    /// focus is drawn as focused and gets the keys.
    #[test]
    fn removing_tabs_under_a_focused_bar_that_nobody_can_take_over() {
        let (first, seen_first) = probe("a", false);
        let (second, _) = probe("b", false);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        // The bar takes the focus.
        view.take_focus(Direction::none()).unwrap();
        assert_eq!(view.focus_part(), Some(Part::Bar));
        view.remove_tab(1);
        assert!(
            !view.content_focused(),
            "a view that refuses the focus is drawn as focused"
        );
        send(&mut view, Event::Char('x'));
        assert!(
            seen_first.events.lock().unwrap().is_empty(),
            "key routed to a view that refused the focus"
        );
    }

    #[test]
    fn tiny_sizes_do_not_panic() {
        for placement in [
            TabPlacement::Top,
            TabPlacement::Bottom,
            TabPlacement::Left,
            TabPlacement::Right,
        ] {
            for show_bar in [true, false] {
                for tabs in 0..4 {
                    for with_format in [false, true] {
                        let mut view = TabView::new().bar_placement(placement).show_bar(show_bar);
                        if with_format {
                            view.set_title_format(|t, s| {
                                if s.active {
                                    format!("[{}]", t.source()).into()
                                } else {
                                    t.clone()
                                }
                            });
                        }
                        for i in 0..tabs {
                            view.add_tab(format!("T{i}"), probe("a", i != 1).0);
                        }
                        let mut bar = view.detached_bar();
                        view.set_show_bar(show_bar);
                        for w in 0..4 {
                            for h in 0..4 {
                                let size = Vec2::new(w, h);
                                view.required_size(size);
                                let _ = render(&mut view, size);
                                view.important_area(size);
                                bar.required_size(size);
                                bar.layout(size);
                                let theme = Theme::retro();
                                let mut buffer = PrintBuffer::new();
                                buffer.resize(size + (3, 2));
                                let buffer = RwLock::new(buffer);
                                bar.draw(&Printer::new(size, &theme, &buffer));
                                for x in 0..5 {
                                    for y in 0..5 {
                                        for off in [(0, 0), (1, 1)] {
                                            let _ = send(&mut view, press(off, (x, y)));
                                            let _ = send_to(&mut bar, press(off, (x, y)));
                                        }
                                    }
                                }
                                for key in [
                                    Key::Left,
                                    Key::Right,
                                    Key::Up,
                                    Key::Down,
                                    Key::Tab,
                                    Key::Home,
                                    Key::End,
                                ] {
                                    send(&mut view, key);
                                    send_to(&mut bar, key);
                                    send(&mut view, Event::Shift(Key::Tab));
                                }
                                let _ = view.take_focus(Direction::none());
                                let _ = bar.take_focus(Direction::none());
                                view.layout_key();
                                bar.layout_key();
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn two_bar_switches_before_layout() {
        let (a, sa) = probe("a", true);
        let (b, sb) = probe("b", true);
        let (c, sc) = probe("c", true);
        let mut view = TabView::new().tab("A", a).tab("B", b).tab("C", c);
        let mut bar = view.detached_bar();
        view.layout(Vec2::new(6, 3));
        bar.layout(Vec2::new(5, 3));
        view.take_focus(Direction::none()).unwrap();
        send_to(&mut bar, Key::Down);
        send_to(&mut bar, Key::Down);
        view.layout(Vec2::new(6, 3));
        assert_eq!(lost_count(&sa), 1);
        assert_eq!(lost_count(&sb), 0);
        assert!(sb.sources.lock().unwrap().is_empty());
        assert_eq!(sc.sources.lock().unwrap().len(), 1);
        // And back to the original one: nothing.
        send_to(&mut bar, Key::Up);
        send_to(&mut bar, Key::Up);
        view.layout(Vec2::new(6, 3));
        assert_eq!(lost_count(&sc), 1);
        assert_eq!(sa.sources.lock().unwrap().len(), 2);
    }

    #[test]
    fn insert_live_tabs_before_and_at_the_active_one() {
        let n = Arc::new(AtomicUsize::new(0));
        let mut view = TabView::new();
        view.add_tab("plain", probe("p", true).0);
        let a = Arc::clone(&n);
        view.add_tab_live(
            move || format!("live{}", a.load(Ordering::Relaxed)),
            probe("l", true).0,
        );
        view.set_active(1).unwrap();
        // Insert plain at active index: active moves to 2, the live closure with it.
        view.insert_tab(1, "ins", probe("i", true).0);
        assert_eq!(view.active(), Some(2));
        let b = Arc::clone(&n);
        view.insert_tab_live(
            0,
            move || format!("first{}", b.load(Ordering::Relaxed) + 100),
            probe("f", true).0,
        );
        assert_eq!(view.active(), Some(3));
        view.layout(Vec2::new(40, 3));
        n.store(7, Ordering::Relaxed);
        view.layout(Vec2::new(40, 3));
        let titles: Vec<String> = view
            .shared
            .lock()
            .titles
            .iter()
            .map(|t| t.source().to_string())
            .collect();
        assert_eq!(titles, ["first107", "plain", "ins", "live7"]);
        assert_eq!(letter(&view, 3), "l");
    }

    #[test]
    fn layout_key_is_stable_and_changes_with_the_tabs() {
        let mut view = view(TabPlacement::Top);
        let mut bar = view.detached_bar();
        view.layout(Vec2::new(10, 4));
        bar.layout(Vec2::new(10, 4));
        let (k1, b1) = (view.layout_key(), bar.layout_key());
        assert_eq!((k1, b1), (view.layout_key(), bar.layout_key()));
        // Focus changes do not matter.
        view.take_focus(Direction::none()).unwrap();
        send_to(&mut bar, Key::Down);
        let b2 = bar.layout_key();
        assert_eq!(b1, b2, "switching tabs changes the bar's layout key");
        assert_ne!(k1, view.layout_key());
        assert!(view.needs_relayout());
        view.layout(Vec2::new(10, 4));
        assert!(!view.needs_relayout());
        // Same size: set_title with the same text still changes key (acceptable).
        view.set_title(0, "One");
        assert!(view.needs_relayout());
        view.layout(Vec2::new(10, 4));
        view.set_bar_align(HAlign::Right);
        // Placement
        view.set_bar_placement(TabPlacement::Left);
        assert!(view.needs_relayout());
    }

    #[test]
    fn formatter_with_a_title_change_between_size_and_draw() {
        let mut view = TabView::new()
            .tab("A", probe("a", true).0)
            .tab("B", probe("b", true).0)
            .title_format(|t, s| {
                if s.active {
                    format!("[{}]", t.source()).into()
                } else {
                    format!(" {} ", t.source()).into()
                }
            });
        let size = view.required_size(Vec2::new(40, 3));
        view.layout(size);
        view.set_title(0, "LONGER");
        let rows = {
            let buffer = {
                let theme = Theme::retro();
                let mut buffer = PrintBuffer::new();
                buffer.resize(Vec2::new(40, 3));
                let buffer = RwLock::new(buffer);
                view.draw(&Printer::new(Vec2::new(40, 3), &theme, &buffer));
                buffer
            };
            let buffer = buffer.read();
            (0..40)
                .map(|x| buffer.cell_text(Vec2::new(x, 0)).unwrap_or(".").to_string())
                .collect::<String>()
        };
        // Expect "[LONGER] B " drawn with formatter; without formatter it would be " LONGER  B ".
        assert!(rows.starts_with("[LONGER] B "), "{rows}");
    }

    #[test]
    fn mouse_hits_the_right_title_at_the_boundaries() {
        let mut view = view(TabPlacement::Top); // cells: 0..5, 5..10, 10..15
        view.layout(Vec2::new(20, 3));
        send(&mut view, press((0, 0), (4, 0)));
        assert_eq!(view.active(), Some(0));
        send(&mut view, press((0, 0), (5, 0)));
        assert_eq!(view.active(), Some(1));
        send(&mut view, press((0, 0), (14, 0)));
        assert_eq!(view.active(), Some(2));
        assert!(!send(&mut view, press((0, 0), (15, 0))) || view.active() == Some(2));
    }

    #[test]
    fn mouse_in_an_overflowing_bar_with_the_last_tab_active() {
        let mut view = TabView::new();
        for (t, l) in [("AAA", "a"), ("BBB", "b"), ("CCC", "c"), ("DDD", "d")] {
            view.add_tab(t, probe(l, true).0);
        }
        view.set_active(3).unwrap();
        view.layout(Vec2::new(12, 3)); // two cells of 5 fit, a third would stick out
        let rows = render(&mut view, (12, 2));
        assert_eq!(rows[0], "_CCC__DDD_..", "{rows:?}");
        send(&mut view, press((0, 0), (1, 0)));
        assert_eq!(view.active(), Some(2));
        // Now active is 2: start = 1? B C D
        let rows = render(&mut view, (12, 2));
        assert_eq!(rows[0], "_BBB__CCC__D", "{rows:?}");
    }

    #[test]
    fn remove_the_active_tab_with_a_pending_bar_switch() {
        let (a, sa) = probe("a", true);
        let (b, sb) = probe("b", true);
        let (c, sc) = probe("c", true);
        let mut view = TabView::new().tab("A", a).tab("B", b).tab("C", c);
        let mut bar = view.detached_bar();
        view.layout(Vec2::new(6, 3));
        bar.layout(Vec2::new(5, 3));
        view.take_focus(Direction::none()).unwrap();
        send_to(&mut bar, Key::Down);
        send_to(&mut bar, Key::Down); // active = 2 pending
        view.remove_tab(2);
        assert_eq!(view.active(), Some(1));
        assert_eq!(lost_count(&sa), 1);
        assert_eq!(lost_count(&sb), 0);
        assert_eq!(lost_count(&sc), 0);
        assert!(view.content_focused());
        assert_eq!(sb.sources.lock().unwrap().len(), 1);
        assert_eq!(bar.last_size, Vec2::new(5, 3));
    }

    #[test]
    fn zero_tabs_and_back_keeps_the_focus() {
        let mut view = TabView::new().tab("One", probe("a", true).0);
        view.take_focus(Direction::none()).unwrap();
        assert!(view.content_focused());
        view.remove_tab(0);
        let (new, seen_new) = probe("n", true);
        view.add_tab("New", new);
        // The view is still the one the parent gave the focus to.
        assert!(view.content_focused(), "content_unfocused stayed set");
        send(&mut view, Event::Char('x'));
        assert_eq!(seen_new.events.lock().unwrap().len(), 1);
    }

    fn take_count(seen: &Seen) -> usize {
        seen.sources.lock().unwrap().len()
    }

    #[test]
    fn pending_callback_goes_with_the_next_consumed_event() {
        struct Notify(Arc<AtomicUsize>);
        impl View for Notify {
            fn draw(&self, _: &Printer) {}
            fn required_size(&mut self, _: Vec2) -> Vec2 {
                Vec2::new(1, 1)
            }
            fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
                Ok(EventResult::Consumed(None))
            }
            fn on_event(&mut self, event: Event) -> EventResult {
                match event {
                    Event::FocusLost => {
                        let count = Arc::clone(&self.0);
                        EventResult::with_cb(move |_| {
                            count.fetch_add(1, Ordering::Relaxed);
                        })
                    }
                    Event::Char('y') => EventResult::consumed(),
                    _ => EventResult::Ignored,
                }
            }
        }
        let count = Arc::new(AtomicUsize::new(0));
        let mut view = TabView::new()
            .tab("One", Notify(Arc::clone(&count)))
            .tab("Two", Notify(Arc::clone(&count)));
        view.take_focus(Direction::back()).unwrap();
        view.set_active(1).unwrap();
        // Ignored once: the key is not swallowed, and the callback waits.
        assert!(!send(&mut view, Event::Char('x')));
        assert_eq!(count.load(Ordering::Relaxed), 0);
        // Consumed: the callback goes with it, once.
        assert!(send(&mut view, Event::Char('y')));
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert!(!send(&mut view, Event::Char('x')));
        assert!(send(&mut view, Event::Char('y')));
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_tab_change_while_the_focus_is_away_offers_it_to_the_new_view_later() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", true);
        let mut view = TabView::new().tab("One", first).tab("Two", second);
        view.take_focus(Direction::back()).unwrap();
        send(&mut view, Event::FocusLost);
        view.set_active(1).unwrap();
        assert!(!view.content_focused());
        assert_eq!((lost_count(&seen_first), take_count(&seen_second)), (1, 0));

        view.take_focus(Direction::none()).unwrap();
        assert!(view.content_focused());
        assert_eq!((lost_count(&seen_first), take_count(&seen_second)), (1, 1));
        send(&mut view, Event::Char('x'));
        assert_eq!(seen_second.events.lock().unwrap().len(), 1);
        send(&mut view, Event::FocusLost);
        assert_eq!(lost_count(&seen_second), 1);
        // Not told again.
        send(&mut view, Event::FocusLost);
        assert_eq!(lost_count(&seen_second), 1);
    }

    #[test]
    fn a_single_view_that_refuses_the_focus_is_not_given_keys() {
        let (only, seen) = probe("a", false);
        let mut view = TabView::new().tab("One", only);
        assert!(view.take_focus(Direction::none()).is_err());
        assert!(!view.content_focused());
        send(&mut view, Event::Char('x'));
        assert!(seen.events.lock().unwrap().is_empty());
    }

    /// A detached bar and its `TabView` in a `LinearLayout`, driven only through the layout.
    #[test]
    fn detached_bar_in_a_linear_layout_end_to_end() {
        let (first, seen_first) = probe("a", true);
        let (second, seen_second) = probe("b", true);
        let mut pages = TabView::new().tab("One", first).tab("Two", second);
        let bar = pages.detached_bar();
        let mut layout = LinearLayout::horizontal().child(bar).child(pages);
        layout.layout(Vec2::new(30, 6));

        // The bar is first: it gets the focus, then the pages.
        layout.take_focus(Direction::none()).unwrap();
        send_to(&mut layout, Key::Right);
        assert_eq!(take_count(&seen_first), 1);
        assert_eq!(lost_count(&seen_first), 0);

        // Back to the bar: the content is told, once.
        send_to(&mut layout, Key::Left);
        assert_eq!(lost_count(&seen_first), 1);

        // Switch tab from the bar, and come back to the pages.
        assert!(send_to(&mut layout, Key::Down));
        layout.layout(Vec2::new(30, 6));
        assert_eq!(lost_count(&seen_first), 1, "told twice");
        assert_eq!(
            take_count(&seen_second),
            0,
            "offered the focus it cannot get"
        );
        send_to(&mut layout, Key::Right);
        assert_eq!(take_count(&seen_second), 1);
        assert_eq!(take_count(&seen_first), 1);

        // Keys reach the new tab, and not the old one.
        send_to(&mut layout, Event::Char('x'));
        assert_eq!(seen_second.events.lock().unwrap().len(), 1);
        assert!(
            !seen_first
                .events
                .lock()
                .unwrap()
                .contains(&Event::Char('x'))
        );

        // And back again: only the new view is told.
        send_to(&mut layout, Key::Left);
        assert_eq!((lost_count(&seen_first), lost_count(&seen_second)), (1, 1));
        send_to(&mut layout, Key::Up);
        send_to(&mut layout, Key::Right);
        assert_eq!(take_count(&seen_first), 2);
        assert_eq!(take_count(&seen_second), 1);
    }

    #[test]
    fn release_left_of_or_above_the_view_reaches_the_focused_content() {
        let (content, seen) = probe("a", true);
        let mut view = TabView::new().tab("One", content).show_bar(false);
        view.layout(Vec2::new(10, 5));
        view.take_focus(Direction::none()).unwrap();
        // A drag that started in the content, released above/left of the view (offset (3, 4)).
        for position in [(5, 5), (1, 5), (5, 1)] {
            let before = seen.events.lock().unwrap().len();
            send(
                &mut view,
                Event::Mouse {
                    offset: Vec2::new(3, 4),
                    position: Vec2::new(position.0, position.1),
                    event: MouseEvent::Release(MouseButton::Left),
                },
            );
            assert_eq!(
                seen.events.lock().unwrap().len(),
                before + 1,
                "release at {position:?} was dropped"
            );
        }
    }

    #[test]
    fn linear_layout_forwards_a_release_left_of_the_view() {
        let (content, seen) = probe("a", true);
        let mut layout = LinearLayout::vertical().child(content);
        layout.layout(Vec2::new(10, 5));
        layout.take_focus(Direction::none()).unwrap();
        send_to(
            &mut layout,
            Event::Mouse {
                offset: Vec2::new(3, 4),
                position: Vec2::new(1, 5),
                event: MouseEvent::Release(MouseButton::Left),
            },
        );
        assert_eq!(seen.events.lock().unwrap().len(), 1);
    }

    #[test]
    fn orphan_bar_stops_evaluating_the_closures_of_a_dropped_view() {
        let n = Arc::new(AtomicUsize::new(0));
        let mut view = TabView::new().tab("A", probe("a", true).0);
        let counter = Arc::clone(&n);
        view.add_tab_live(
            move || format!("live{}", counter.load(Ordering::Relaxed)),
            probe("b", true).0,
        );
        let mut bar = view.detached_bar();
        bar.layout(Vec2::new(8, 2));
        drop(view);
        let seen = n.fetch_add(1, Ordering::Relaxed);
        let _ = seen;
        bar.layout(Vec2::new(8, 2));
        let titles: Vec<String> = bar
            .shared
            .lock()
            .titles
            .iter()
            .map(|t| t.source().to_string())
            .collect();
        assert_eq!(
            titles,
            ["A", "live0"],
            "the orphan bar still runs the closures of a dropped view"
        );
    }

    #[test]
    fn a_dropped_view_releases_what_its_live_titles_captured() {
        let captured = Arc::new(AtomicUsize::new(0));
        let mut view = TabView::new().tab("A", probe("a", true).0);
        let counter = Arc::clone(&captured);
        view.add_tab_live(
            move || format!("n{}", counter.load(Ordering::Relaxed)),
            probe("b", true).0,
        );
        let mut bar = view.detached_bar();
        bar.layout(Vec2::new(8, 2));
        assert_eq!(Arc::strong_count(&captured), 2);
        drop(view);
        assert_eq!(Arc::strong_count(&captured), 1);
        // And the orphan still draws what it knew.
        bar.layout(Vec2::new(8, 2));
        assert_eq!(bar.shared.lock().titles[1].source(), "n0");
    }
}

#[cfg(test)]
mod live_title_tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::theme::Theme;
    use crate::view::layout_pass::in_layout_pass;
    use crate::views::TextView;
    use parking_lot::RwLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A counter, and a closure printing it after `name`, counting its calls.
    fn counting(name: &'static str) -> (Arc<AtomicUsize>, Arc<AtomicUsize>, impl Fn() -> String) {
        let value = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let (v, c) = (Arc::clone(&value), Arc::clone(&calls));
        let title = move || {
            c.fetch_add(1, Ordering::Relaxed);
            format!("{name} ({})", v.load(Ordering::Relaxed))
        };
        (value, calls, title)
    }

    fn text() -> TextView {
        TextView::new("content")
    }

    /// Lays out the view, draws it, and gives the text of one line (`_` for spaces).
    fn line(view: &mut impl View, size: (usize, usize), y: usize) -> String {
        let size = Vec2::from(size);
        view.layout(size);
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        let buffer = RwLock::new(buffer);
        let theme = Theme::retro();
        view.draw(&Printer::new(size, &theme, &buffer));
        let buffer = buffer.read();
        (0..size.x)
            .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                None | Some("") => ".",
                Some(" ") => "_",
                Some(text) => text,
            })
            .collect()
    }

    fn calls(calls: &AtomicUsize) -> usize {
        calls.load(Ordering::Relaxed)
    }

    #[test]
    fn builtin_bar_follows_the_closure() {
        let (value, count, title) = counting("Logs");
        let mut view = TabView::new().tab("Home", text()).tab_live(title, text());
        assert_eq!(line(&mut view, (20, 2), 0), "_Home__Logs_(0)_....");
        value.store(3, Ordering::Relaxed);
        assert_eq!(line(&mut view, (20, 2), 0), "_Home__Logs_(3)_....");
        assert!(calls(&count) >= 2);
    }

    #[test]
    fn detached_bar_follows_the_closure() {
        let (value, _, title) = counting("Logs");
        let mut view = TabView::new().tab("Home", text()).tab_live(title, text());
        let mut bar = view.detached_bar();
        assert_eq!(line(&mut bar, (8, 2), 1), "_Logs_(0");
        value.store(12, Ordering::Relaxed);
        assert_eq!(line(&mut bar, (9, 2), 1), "_Logs_(12");
        // The bar is wide enough for the new title.
        assert_eq!(bar.required_size(Vec2::new(40, 40)), Vec2::new(11, 2));
    }

    #[test]
    fn evaluated_once_per_pass() {
        let (_, count, title) = counting("Logs");
        let mut view = TabView::new().tab("Home", text()).tab_live(title, text());
        let mut one = view.detached_bar();
        let mut two = view.detached_bar().orientation(Orientation::Horizontal);
        let size = Vec2::new(20, 4);

        let mut pass = || {
            in_layout_pass(|| {
                one.needs_relayout();
                two.needs_relayout();
                view.needs_relayout();
                one.layout_key();
                two.layout_key();
                view.layout_key();
                one.required_size(size);
                two.required_size(size);
                view.required_size(size);
                one.layout(size);
                two.layout(size);
                view.layout(size);
            });
        };

        pass();
        assert_eq!(calls(&count), 1);
        pass();
        assert_eq!(calls(&count), 2);
        pass();
        assert_eq!(calls(&count), 3);

        // Outside of a pass, every call may evaluate again.
        two.layout_key();
        two.layout_key();
        assert_eq!(calls(&count), 5);
    }

    #[test]
    fn the_closure_does_not_run_to_draw_or_to_build() {
        let (_, count, title) = counting("Logs");
        let mut view = TabView::new().tab("Home", text()).tab_live(title, text());
        assert_eq!(calls(&count), 0);
        view.layout(Vec2::new(20, 2));
        let before = calls(&count);
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(20, 2));
        let buffer = RwLock::new(buffer);
        let theme = Theme::retro();
        view.draw(&Printer::new((20, 2), &theme, &buffer));
        assert_eq!(calls(&count), before);
    }

    #[test]
    fn a_tab_view_without_live_titles_has_nothing_to_evaluate() {
        let view = TabView::new().tab("A", text()).tab("B", text());
        assert_eq!(view.shared.lock().live_count, 0);
        // No pass is recorded either.
        in_layout_pass(|| {
            view.needs_relayout();
            assert_eq!(view.shared.lock().evaluated_in, None);
        });
    }

    #[test]
    fn equal_text_changes_nothing() {
        let (value, _, title) = counting("Logs");
        let mut view = TabView::new().tab("Home", text()).tab_live(title, text());
        let mut bar = view.detached_bar();
        bar.layout(Vec2::new(20, 2));
        view.layout(Vec2::new(20, 2));
        let (bar_key, view_key) = (bar.layout_key(), view.layout_key());
        let titles_key = view.shared.lock().titles_key;
        assert!(!bar.needs_relayout());
        assert!(!view.needs_relayout());

        // Evaluated again, but the same text.
        assert_eq!(bar.layout_key(), bar_key);
        assert_eq!(view.layout_key(), view_key);
        assert_eq!(view.shared.lock().titles_key, titles_key);

        let size = bar.required_size(Vec2::new(40, 40));
        value.store(10, Ordering::Relaxed);
        assert!(bar.needs_relayout());
        assert!(view.needs_relayout());
        assert_ne!(bar.layout_key(), bar_key);
        assert_ne!(view.layout_key(), view_key);
        assert_ne!(bar.required_size(Vec2::new(40, 40)), size);
    }

    #[test]
    fn set_title_disables_the_closure() {
        let (value, count, title) = counting("Logs");
        let mut view = TabView::new().tab("Home", text()).tab_live(title, text());
        view.layout(Vec2::new(20, 2));
        assert_eq!(view.shared.lock().live_count, 1);
        let before = calls(&count);

        view.set_title(1, "Fixed");
        assert_eq!(view.shared.lock().live_count, 0);
        value.store(7, Ordering::Relaxed);
        assert_eq!(line(&mut view, (20, 2), 0), "_Home__Fixed_.......");
        assert_eq!(line(&mut view, (20, 2), 0), "_Home__Fixed_.......");
        assert_eq!(calls(&count), before);

        // Setting the title of a plain tab changes nothing about the closures.
        view.set_title(0, "Start");
        assert_eq!(view.shared.lock().live_count, 0);
        // Out of range.
        view.set_title(9, "Nope");
    }

    #[test]
    fn set_title_source_on_a_plain_tab() {
        let (value, count, title) = counting("Home");
        let mut view = TabView::new().tab("Home", text()).tab("Logs", text());
        let mut bar = view.detached_bar();
        view.set_title_source(0, title);
        assert_eq!(view.shared.lock().live_count, 1);
        // The current title stays until the closure first runs.
        assert_eq!(view.shared.lock().titles[0].source(), "Home");
        assert_eq!(calls(&count), 0);

        value.store(4, Ordering::Relaxed);
        assert_eq!(line(&mut bar, (10, 2), 0), "_Home_(4)_");
        assert_eq!(line(&mut bar, (10, 2), 1), "_Logs_____");

        // Replacing a closure does not count it twice.
        view.set_title_source(0, || "Other");
        assert_eq!(view.shared.lock().live_count, 1);
        assert_eq!(line(&mut bar, (10, 2), 0), "_Other____");

        // Out of range: nothing happens.
        view.set_title_source(5, || "Nope");
        assert_eq!(view.shared.lock().live_count, 1);
    }

    #[test]
    fn closures_stay_with_their_tab() {
        let mut view = TabView::new()
            .tab_live(|| "a", text())
            .tab("b", text())
            .tab_live(|| "c", text());
        view.layout(Vec2::new(20, 2));

        view.insert_tab_live(1, || "x", text());
        view.insert_tab(0, "plain", text());
        view.layout(Vec2::new(30, 2));
        {
            let shared = view.shared.lock();
            let titles: Vec<&str> = shared.titles.iter().map(|t| t.source()).collect();
            assert_eq!(titles, ["plain", "a", "x", "b", "c"]);
            assert_eq!(shared.live_count, 3);
        }

        // Removing a plain tab, then a live one.
        view.remove_tab(3);
        view.remove_tab(1);
        view.layout(Vec2::new(30, 2));
        let shared = view.shared.lock();
        let titles: Vec<&str> = shared.titles.iter().map(|t| t.source()).collect();
        assert_eq!(titles, ["plain", "x", "c"]);
        let live: Vec<bool> = shared.sources.iter().map(Option::is_some).collect();
        assert_eq!(live, [false, true, true]);
        assert_eq!(shared.live_count, 2);
    }

    #[test]
    fn closures_run_with_the_lock_released() {
        let mut view = TabView::new().tab("Home", text()).tab("Logs", text());
        let shared = Arc::clone(&view.shared);
        let free = Arc::new(AtomicUsize::new(0));
        let total = Arc::new(AtomicUsize::new(0));
        let (f, t) = (Arc::clone(&free), Arc::clone(&total));
        view.set_title_source(1, move || {
            t.fetch_add(1, Ordering::Relaxed);
            // `parking_lot` mutexes are not reentrant: this fails if the lock is held.
            if shared.try_lock().is_some() {
                f.fetch_add(1, Ordering::Relaxed);
            }
            "Logs"
        });
        let bar = view.detached_bar();
        in_layout_pass(|| {
            bar.layout_key();
            view.layout(Vec2::new(20, 2));
        });
        assert_eq!(calls(&total), 1);
        assert_eq!(calls(&free), 1);
    }
}

#[cfg(test)]
mod title_format_tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::style::{Color, ConcreteStyle, Style};
    use crate::theme::Theme;
    use crate::view::layout_pass::in_layout_pass;
    use crate::views::TextView;
    use parking_lot::RwLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn text() -> TextView {
        TextView::new("content")
    }

    /// Brackets around the active title, spaces around the others.
    fn brackets(title: &StyledString, state: TabState) -> StyledString {
        if state.active {
            format!("[{}]", title.source()).into()
        } else {
            format!(" {} ", title.source()).into()
        }
    }

    fn two_tabs(placement: TabPlacement) -> TabView {
        TabView::new()
            .bar_placement(placement)
            .tab("One", text())
            .tab("Two", text())
            .title_format(brackets)
    }

    fn draw(
        view: &mut impl View,
        size: (usize, usize),
        focused: bool,
        theme: &Theme,
    ) -> RwLock<PrintBuffer> {
        let size = Vec2::from(size);
        view.layout(size);
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        let buffer = RwLock::new(buffer);
        view.draw(&Printer::new(size, theme, &buffer).focused(focused));
        buffer
    }

    /// Text of a line (`_` for spaces, `.` for cells not printed).
    fn text_line(buffer: &RwLock<PrintBuffer>, y: usize) -> String {
        let buffer = buffer.read();
        (0..buffer.size().x)
            .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                None | Some("") => ".",
                Some(" ") => "_",
                Some(text) => text,
            })
            .collect()
    }

    fn line(view: &mut impl View, size: (usize, usize), y: usize) -> String {
        text_line(&draw(view, size, false, &Theme::retro()), y)
    }

    fn style_at(buffer: &RwLock<PrintBuffer>, x: usize, y: usize) -> ConcreteStyle {
        buffer.read().cell_style(Vec2::new(x, y)).unwrap()
    }

    /// `(tab, offset, length)` of the cells of a horizontal bar, for the titles of a view.
    fn cells(view: &TabView, available: usize) -> Vec<(usize, usize, usize)> {
        let layout = BarLayout {
            orientation: Orientation::Horizontal,
            align: HAlign::Left,
        };
        with_titles(&view.shared, |titles, active| {
            layout.title_cells(titles, active, available)
        })
    }

    #[test]
    fn brackets_in_each_bar() {
        // Built-in bar, horizontal. Every cell is 5 wide.
        let mut view = two_tabs(TabPlacement::Top);
        assert_eq!(line(&mut view, (12, 2), 0), "[One]_Two_..");
        view.set_active(1).unwrap();
        assert_eq!(line(&mut view, (12, 2), 0), "_One_[Two]..");

        // Built-in bar, vertical.
        let mut view = two_tabs(TabPlacement::Left);
        let buffer = draw(&mut view, (9, 2), false, &Theme::retro());
        assert_eq!(&text_line(&buffer, 0)[..5], "[One]");
        assert_eq!(&text_line(&buffer, 1)[..5], "_Two_");

        // Detached bars.
        let mut view = two_tabs(TabPlacement::Top);
        let mut vertical = view.detached_bar();
        let mut horizontal = view.detached_bar().orientation(Orientation::Horizontal);
        let buffer = draw(&mut vertical, (5, 2), false, &Theme::retro());
        assert_eq!(text_line(&buffer, 0), "[One]");
        assert_eq!(text_line(&buffer, 1), "_Two_");
        assert_eq!(line(&mut horizontal, (10, 1), 0), "[One]_Two_");
        assert_eq!(vertical.required_size(Vec2::new(40, 40)), Vec2::new(5, 2));
        assert_eq!(
            horizontal.required_size(Vec2::new(40, 40)),
            Vec2::new(10, 1)
        );
    }

    #[test]
    fn a_focus_marker_does_not_move_anything() {
        let mut view = TabView::new()
            .tab("One", text())
            .tab("Two", text())
            .title_format(|title, state| {
                if state.active && state.focused {
                    format!("*{}", title.source()).into()
                } else {
                    title.clone()
                }
            });
        view.layout(Vec2::new(20, 2));
        let before = cells(&view, 20);
        let theme = Theme::retro();
        let focused = draw(&mut view, (20, 2), true, &theme);
        let unfocused = draw(&mut view, (20, 2), false, &theme);
        assert_eq!(&text_line(&focused, 0)[..8], "*OneTwo_");
        assert_eq!(&text_line(&unfocused, 0)[..8], "One_Two_");
        // The same cells, even though the text changed.
        assert_eq!(cells(&view, 20), before);
        assert_eq!(before, [(0, 0, 4), (1, 4, 4)]);
    }

    #[test]
    fn cells_are_as_wide_as_the_widest_rendering() {
        // Widths: inactive 1, active 3, active with the focus 5, inactive with the focus 2.
        let view = TabView::new().tab("T", text()).title_format(|_, state| {
            let width = match (state.active, state.focused) {
                (false, false) => 1,
                (true, false) => 3,
                (true, true) => 5,
                (false, true) => 2,
            };
            "x".repeat(width).into()
        });
        assert_eq!(cells(&view, 20), [(0, 0, 5)]);

        // Without the focus variants, it is the active one.
        let view = TabView::new()
            .tab("T", text())
            .title_format(|_, state| if state.active { "abcd" } else { "a" }.into());
        assert_eq!(cells(&view, 20), [(0, 0, 4)]);
    }

    #[test]
    fn the_formatter_gets_the_index() {
        let view = TabView::new()
            .tab("a", text())
            .tab("b", text())
            .tab("c", text())
            .title_format(|title, state| format!("{}{}", title.source(), state.index).into());
        let mut bar = { view }.detached_bar();
        assert_eq!(line(&mut bar, (2, 3), 2), "c2");
    }

    #[test]
    fn style_keys_apply_over_the_formatted_title() {
        let theme = Theme::retro();
        let highlight = {
            let mut view = TabView::new().tab("A", text()).tab("B", text());
            // Without a formatter the whole cell has the style.
            let buffer = draw(&mut view, (8, 2), true, &theme);
            style_at(&buffer, 1, 0)
        };
        let mut view = two_tabs(TabPlacement::Top);
        let buffer = draw(&mut view, (12, 2), true, &theme);
        // The whole cell of the active tab, brackets included.
        for x in 0..5 {
            assert_eq!(style_at(&buffer, x, 0), highlight, "x = {x}");
        }
        assert_ne!(style_at(&buffer, 6, 0), highlight);

        // A neutral style gives the brackets and no highlight.
        let mut neutral = Theme::retro();
        neutral.palette.set_custom_style(&TAB_ACTIVE, Style::none());
        neutral
            .palette
            .set_custom_style(&TAB_INACTIVE, Style::none());
        let buffer = draw(&mut view, (12, 2), true, &neutral);
        assert_eq!(text_line(&buffer, 0), "[One]_Two_..");
        assert_eq!(style_at(&buffer, 1, 0), style_at(&buffer, 6, 0));
        assert_ne!(style_at(&buffer, 1, 0), highlight);
    }

    #[test]
    fn clicks_in_the_padding_select_the_tab() {
        // The inactive rendering is narrower than the cell: the end of the cell is padding.
        let mut view = TabView::new()
            .tab("One", text())
            .tab("Two", text())
            .tab("Six", text())
            .title_format(|title, state| {
                if state.active {
                    format!("[{}]", title.source()).into()
                } else {
                    title.clone()
                }
            });
        view.layout(Vec2::new(20, 3));
        assert_eq!(cells(&view, 20), [(0, 0, 5), (1, 5, 5), (2, 10, 5)]);
        let click = |view: &mut TabView, x: usize| {
            let event = Event::Mouse {
                offset: Vec2::zero(),
                position: Vec2::new(x, 0),
                event: MouseEvent::Press(MouseButton::Left),
            };
            view.on_event(event).process(&mut Cursive::new());
            view.active()
        };
        // The last cell of the second tab, where nothing is printed.
        assert_eq!(click(&mut view, 9), Some(1));
        assert_eq!(click(&mut view, 14), Some(2));
        assert_eq!(click(&mut view, 0), Some(0));
        assert_eq!(click(&mut view, 4), Some(0));
        assert_eq!(click(&mut view, 5), Some(1));
    }

    #[test]
    fn setting_and_clearing_the_formatter_remeasures() {
        let mut view = TabView::new().tab("One", text()).tab("Two", text());
        let mut bar = view.detached_bar();
        bar.layout(Vec2::new(10, 2));
        view.layout(Vec2::new(10, 2));
        let (bar_key, view_key) = (bar.layout_key(), view.layout_key());
        let size = bar.required_size(Vec2::new(40, 40));
        assert_eq!(size, Vec2::new(5, 2));
        assert!(!bar.needs_relayout());

        view.set_title_format(|title, _| format!("<<{}>>", title.source()).into());
        assert!(bar.needs_relayout());
        assert!(view.needs_relayout());
        assert_ne!(bar.layout_key(), bar_key);
        assert_ne!(view.layout_key(), view_key);
        assert_eq!(bar.required_size(Vec2::new(40, 40)), Vec2::new(7, 2));
        assert_eq!(line(&mut bar, (7, 2), 0), "<<One>>");

        let formatted_key = bar.layout_key();
        view.clear_title_format();
        assert_ne!(bar.layout_key(), formatted_key);
        assert_eq!(bar.required_size(Vec2::new(40, 40)), size);
        assert_eq!(line(&mut bar, (5, 2), 0), "_One_");

        // Clearing nothing changes nothing.
        bar.layout(Vec2::new(5, 2));
        view.layout(Vec2::new(5, 2));
        view.clear_title_format();
        assert!(!bar.needs_relayout());
    }

    #[test]
    fn styled_output_keeps_its_styles() {
        let red = Color::Rgb(255, 0, 0);
        let mut view = TabView::new()
            .tab("One", text())
            .tab("Two", text())
            .title_format(move |title, _| {
                let mut styled = StyledString::styled("#", Style::from(red));
                styled.append(title.clone());
                styled
            });
        view.set_active(1).unwrap();
        let theme = Theme::retro();
        let buffer = draw(&mut view, (10, 2), true, &theme);
        assert_eq!(text_line(&buffer, 0)[..8].to_string(), "#One#Two");
        // The inactive title keeps its color.
        assert_eq!(style_at(&buffer, 0, 0).color.front, red);
        // The active style wins where it conflicts.
        assert_ne!(style_at(&buffer, 4, 0).color.front, red);
    }

    #[test]
    fn empty_for_inactive_tabs() {
        let mut view = TabView::new()
            .tab("One", text())
            .tab("Two", text())
            .title_format(|title, state| {
                if state.active {
                    format!("[{}]", title.source()).into()
                } else {
                    StyledString::new()
                }
            });
        assert_eq!(cells(&view, 20), [(0, 0, 5), (1, 5, 5)]);
        assert_eq!(line(&mut view, (12, 2), 0), "[One]_____..");
        view.set_active(1).unwrap();
        assert_eq!(line(&mut view, (12, 2), 0), "_____[Two]..");
    }

    #[test]
    fn scrolls_to_keep_the_active_cell_visible() {
        let mut view = two_tabs(TabPlacement::Top);
        view.add_tab("Six", text());
        view.set_active(2).unwrap();
        // 7 cells: only the active one fits.
        assert_eq!(cells(&view, 7), [(2, 0, 5)]);
        assert_eq!(line(&mut view, (7, 2), 0), "[Six]..");
        // 10 cells: the active one and the one before.
        assert_eq!(cells(&view, 10), [(1, 0, 5), (2, 5, 5)]);
        assert_eq!(line(&mut view, (10, 2), 0), "_Two_[Six]");
        view.set_active(0).unwrap();
        assert_eq!(cells(&view, 7), [(0, 0, 5), (1, 5, 5)]);
    }

    #[test]
    fn live_titles_are_formatted() {
        let value = Arc::new(AtomicUsize::new(1));
        let v = Arc::clone(&value);
        let mut view = TabView::new()
            .tab("Home", text())
            .tab_live(
                move || format!("Logs {}", v.load(Ordering::Relaxed)),
                text(),
            )
            .title_format(brackets);
        view.set_active(1).unwrap();
        assert_eq!(line(&mut view, (20, 2), 0), "_Home_[Logs_1]......");
        value.store(10, Ordering::Relaxed);
        assert_eq!(line(&mut view, (20, 2), 0), "_Home_[Logs_10].....");
    }

    #[test]
    fn the_formatter_runs_when_measuring_and_not_under_the_lock() {
        let mut view = TabView::new().tab("One", text()).tab("Two", text());
        let shared = Arc::clone(&view.shared);
        let calls = Arc::new(AtomicUsize::new(0));
        let free = Arc::new(AtomicUsize::new(0));
        let (c, f) = (Arc::clone(&calls), Arc::clone(&free));
        view.set_title_format(move |title, _| {
            c.fetch_add(1, Ordering::Relaxed);
            // `parking_lot` mutexes are not reentrant: this fails if the lock is held.
            if shared.try_lock().is_some() {
                f.fetch_add(1, Ordering::Relaxed);
            }
            title.clone()
        });
        let mut bar = view.detached_bar();
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        let size = Vec2::new(10, 2);
        in_layout_pass(|| {
            bar.required_size(size);
            bar.layout(size);
            view.layout(size);
        });
        // Four renderings for each of the two titles, once for all bars.
        assert_eq!(calls.load(Ordering::Relaxed), 8);
        assert_eq!(free.load(Ordering::Relaxed), 8);

        // Drawing and measuring again use the saved result.
        draw(&mut bar, (10, 2), true, &Theme::retro());
        draw(&mut view, (10, 2), true, &Theme::retro());
        bar.required_size(size);
        assert_eq!(calls.load(Ordering::Relaxed), 8);

        // A new title invalidates it, and the titles are formatted again.
        view.set_title(0, "Uno");
        bar.required_size(size);
        assert_eq!(calls.load(Ordering::Relaxed), 16);
        assert_eq!(free.load(Ordering::Relaxed), 16);
    }

    #[test]
    fn no_formatter_means_nothing_is_kept() {
        let mut view = TabView::new().tab("One", text()).tab("Two", text());
        assert_eq!(line(&mut view, (12, 2), 0), "_One__Two_..");
        assert!(view.shared.lock().rendered.is_none());
    }
}
