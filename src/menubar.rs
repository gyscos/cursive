use Cursive;
use menu::MenuTree;
use backend::Backend;
use view::MenuPopup;
use view::KeyEventView;
use theme::ColorStyle;
use Printer;
use view::Position;
use event::*;

use std::rc::Rc;

use unicode_width::UnicodeWidthStr;

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

pub struct Menubar {
    pub menus: Vec<(String, Rc<MenuTree>)>,
    pub autohide: bool,
    pub focus: usize,
    state: State,
}

new_default!(Menubar);

impl Menubar {
    pub fn new() -> Self {
        Menubar {
            menus: Vec::new(),
            autohide: true,
            state: State::Inactive,
            focus: 0,
        }
    }

    fn hide(&mut self) {
        self.state = State::Inactive;
        ::B::clear();
    }

    pub fn take_focus(&mut self) {
        self.state = State::Selected;
    }

    pub fn receive_events(&self) -> bool {
        self.state == State::Selected
    }

    pub fn visible(&self) -> bool {
        !self.autohide || self.state != State::Inactive
    }

    pub fn add(&mut self, title: &str, menu: MenuTree) -> &mut Self {
        self.menus.push((title.to_string(), Rc::new(menu)));
        self
    }

    pub fn draw(&mut self, printer: &Printer) {
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

    pub fn on_event(&mut self, event: Event) -> Option<Callback> {
        match event {
            Event::Key(Key::Esc) => self.hide(),
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
                              if self.autohide {
                    1
                } else {
                    0
                });
                // Since the closure will be called multiple times,
                // we also need a new Rc on every call.
                return Some(Rc::new(move |s| {
                    show_child(s, offset, menu.clone())
                }));
            }
            _ => (),
        }
        None
    }
}

fn show_child(s: &mut Cursive, offset: (usize, usize), menu: Rc<MenuTree>) {
    s.screen_mut()
        .add_layer_at(Position::absolute(offset),
                      KeyEventView::new(MenuPopup::new(menu)
                              .on_dismiss(|s| s.select_menubar())
                              .on_action(|s| {
                                  s.menubar().state = State::Inactive
                              }))
                          .register(Key::Right, |s| {
                s.pop_layer();
                // Act as if we sent "Left" then "Enter"
                s.select_menubar();
                s.menubar().on_event(Event::Key(Key::Right));
                if let Some(cb) = s.menubar()
                    .on_event(Event::Key(Key::Down)) {
                    cb(s);
                }
            })
                          .register(Key::Left, |s| {
                s.pop_layer();
                // Act as if we sent "Left" then "Enter"
                s.select_menubar();
                s.menubar().on_event(Event::Key(Key::Left));
                if let Some(cb) = s.menubar()
                    .on_event(Event::Key(Key::Down)) {
                    cb(s);
                }
            }));

}
