use super::Cursive;
use super::Callback;

use std::rc::Rc;

use ncurses;

use focus::FocusChange;

pub trait ToView {
    fn to_view(self) -> Box<View>;
}

impl<'a> ToView for &'a str {
    fn to_view(self) -> Box<View> {
        Box::new(TextView::new(self))
    }
}

pub trait View {
    fn focus_left(&mut self) -> FocusChange { FocusChange::LostFocus }
    fn focus_right(&mut self) -> FocusChange { FocusChange::LostFocus }
    fn focus_bottom(&mut self) -> FocusChange { FocusChange::LostFocus }
    fn focus_top(&mut self) -> FocusChange { FocusChange::LostFocus }

    fn click(&mut self) -> Option<Rc<Box<Callback>>> { None }
}

pub struct TextView {
    content: String,
}

impl TextView {
    pub fn new(content: &str) -> Self {
        TextView {
            content: content.to_string(),
        }
    }
}

impl View for TextView {
}

pub struct Button {
    label: String,
    callback: Rc<Box<Fn(&mut Cursive)>>,
}

impl Button {
    pub fn new<F>(label: &str, callback: F) -> Self
        where F: 'static + Fn(&mut Cursive) {
        Button {
            label: label.to_string(),
            callback: Rc::new(Box::new(callback)),
        }
    }
}

pub struct Dialog<'a> {
    view: Box<View>,
    buttons: Vec<Button>,
    focus: Option<&'a Button>,
}

impl<'a> Dialog<'a> {
    pub fn new<V: 'static + ToView>(view: V) -> Self {
        Dialog{
            view: view.to_view(),
            buttons: Vec::new(),
            focus: None,
        }
    }

    pub fn button<F>(mut self, label: &str, callback: F) -> Self
        where F: 'static + Fn(&mut Cursive) {

        self.buttons.push(Button::new(label, callback));

        self
    }
}

impl<'a> View for Dialog<'a> {
    fn click(&mut self) -> Option<Rc<Box<Callback>>> {
        match self.focus {
            None => return None,
            Some(btn) => return Some(self.buttons[0].callback.clone()),
        }
    }
}

pub struct BackgroundView;

impl View for BackgroundView {
}
