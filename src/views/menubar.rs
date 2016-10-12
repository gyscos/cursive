use Cursive;
use Printer;
use direction;
use event::*;
use menu::MenuTree;

use std::rc::Rc;
use theme::ColorStyle;

use unicode_width::UnicodeWidthStr;
use vec::Vec2;
use view::{Position, View};
use views::{KeyEventView, MenuPopup};

/// Current state of the menubar
#[derive(PartialEq, Debug)]
enum State {
    /// The menubar is inactive.
    Inactive,
    /// The menubar is actively selected.
    ///
    /// It will receive input.
    Selected,
    /// The menubar is still visible, but a submenu is open.
    ///
    /// It will not receive input.
    Submenu,
}

/// Shows a single-line list of items, with pop-up menus when one is selected.
///
/// The [`Cursive`] root already includes a menubar
/// that you just need to configure.
///
/// [`Cursive`]: ../struct.Cursive.html#method.menubar
pub struct Menubar {
    /// Menu items in this menubar.
    pub menus: Vec<(String, Rc<MenuTree>)>,
    /// TODO: move this out of this view.
    pub autohide: bool,
    focus: usize,

    // TODO: make Menubar impl View and take out the State management
    state: State,
}

new_default!(Menubar);

impl Menubar {
    /// Creates a new, empty menubar.
    pub fn new() -> Self {
        Menubar {
            menus: Vec::new(),
            autohide: true,
            state: State::Inactive,
            focus: 0,
        }
    }

    /// Hides the menubar.
    fn hide(&mut self) {
        self.state = State::Inactive;
    }

    /// True if we should be receiving events.
    pub fn receive_events(&self) -> bool {
        self.state == State::Selected
    }

    /// Returns `true` if we should be drawn.
    pub fn visible(&self) -> bool {
        !self.autohide || self.state != State::Inactive
    }

    /// Adds a new item to the menubar.
    ///
    /// The item will use the given title, and on selection, will open a
    /// popup-menu with the given menu tree.
    pub fn add(&mut self, title: &str, menu: MenuTree) -> &mut Self {
        self.menus.push((title.to_string(), Rc::new(menu)));
        self
    }
}

fn show_child(s: &mut Cursive, offset: (usize, usize), menu: Rc<MenuTree>) {
    // Adds a new layer located near the item title with the menu popup.
    // Also adds two key callbacks on this new view, to handle `left` and
    // `right` key presses.
    // (If the view itself listens for a `left` or `right` press, it will
    // consume it before our KeyEventView. This means sub-menus can properly
    // be entered.)
    s.screen_mut()
        .add_layer_at(Position::absolute(offset),
                      KeyEventView::new(MenuPopup::new(menu)
                              .on_dismiss(|s| s.select_menubar())
                              .on_action(|s| {
                                  s.menubar().state = State::Inactive
                              }))
                          .register(Key::Right, |s| {
                s.pop_layer();
                s.select_menubar();
                // Act as if we sent "Right" then "Down"
                s.menubar().on_event(Event::Key(Key::Right)).process(s);
                if let EventResult::Consumed(Some(cb)) = s.menubar()
                    .on_event(Event::Key(Key::Down)) {
                    cb(s);
                }
            })
                          .register(Key::Left, |s| {
                s.pop_layer();
                s.select_menubar();
                // Act as if we sent "Left" then "Down"
                s.menubar().on_event(Event::Key(Key::Left)).process(s);
                if let EventResult::Consumed(Some(cb)) = s.menubar()
                    .on_event(Event::Key(Key::Down)) {
                    cb(s);
                }
            }));

}

impl View for Menubar {
    fn draw(&self, printer: &Printer) {
        // Draw the bar at the top
        printer.with_color(ColorStyle::Primary, |printer| {
            printer.print_hline((0, 0), printer.size.x, " ");
        });

        // TODO: draw the rest
        let mut offset = 1;
        for (i, &(ref title, _)) in self.menus.iter().enumerate() {
            // We don't want to show HighlightInactive when we're not selected,
            // because it's ugly on the menubar.
            let selected = (self.state != State::Inactive) &&
                           (i == self.focus);
            printer.with_selection(selected, |printer| {
                printer.print((offset, 0), &format!(" {} ", title));
                offset += title.width() + 2;
            });
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Esc) => {
                self.hide();
                return EventResult::with_cb(|s| s.clear());
            }
            Event::Key(Key::Left) => {
                if self.focus > 0 {
                    self.focus -= 1
                } else {
                    self.focus = self.menus.len() - 1
                }
            }
            Event::Key(Key::Right) => {
                if self.focus + 1 < self.menus.len() {
                    self.focus += 1
                } else {
                    self.focus = 0
                }
            }
            Event::Key(Key::Down) |
            Event::Key(Key::Enter) => {
                // First, we need a new Rc to send the callback,
                // since we don't know when it will be called.
                let menu = self.menus[self.focus].1.clone();
                self.state = State::Submenu;
                let offset = (self.menus[..self.focus]
                    .iter()
                    .map(|&(ref title, _)| title.width() + 2)
                    .fold(0, |a, b| a + b),
                              if self.autohide { 1 } else { 0 });
                // Since the closure will be called multiple times,
                // we also need a new Rc on every call.
                return EventResult::with_cb(move |s| {
                    show_child(s, offset, menu.clone())
                });
            }
            _ => return EventResult::Ignored,
        }
        EventResult::Consumed(None)
    }

    fn take_focus(&mut self, _: direction::Direction) -> bool {
        self.state = State::Selected;
        true
    }

    fn get_min_size(&mut self, _: Vec2) -> Vec2 {
        // TODO: scroll the options if the screen is too small?

        // We add 2 to the length of every label for marin.
        // Also, we add 1 at the beginning.
        // (See the `draw()` method)
        let width = self.menus
            .iter()
            .map(|&(ref title, _)| title.len() + 2)
            .fold(1, |a, b| a + b);

        Vec2::new(width, 1)
    }
}
