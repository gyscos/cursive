extern crate cursive;

use cursive::align::HAlign;
use cursive::traits::*;
use cursive::views::{Dialog, DummyView, LinearLayout, TextView};
use cursive::Cursive;
use std::sync::mpsc;

// This example uses a LinearLayout to stick multiple views next to each other.
// The example also shows a different setup where MVC is more or less implemented
// The model part of the equation was too small, so it's within the UI as a text item.
// I copied the setup from a blog entry of David Simmons. 
// With respect to linear.rs I added a 'q' or 'Q' to quit (besides the button)
pub struct Ui {
    cursive: Cursive,
    ui_rx: mpsc::Receiver<UiMessage>,
    _ui_tx: mpsc::Sender<UiMessage>,
    controller_tx: mpsc::Sender<ControllerMessage>,
}

pub enum UiMessage {
    Quit()
}

impl Ui {
    // Create a new Ui. It will use the Sender provided 
    // to send message to the controller.
    pub fn new(controller_tx: mpsc::Sender<ControllerMessage>) -> Ui {

        // Normally, this would be part of the model in stead of part of the ui
        let text = "This is a very simple example of linear layout. Two views \
                    are present, a short title above, and this text. The text \
                    has a fixed width, and the title is centered horizontally.";

        let (ui_tx, ui_rx) = mpsc::channel::<UiMessage>();
        let mut ui = Ui {
            cursive: Cursive::default(),
            ui_rx: ui_rx,
            _ui_tx: ui_tx,
            controller_tx: controller_tx,
        };

        // Create a dialog with a TextView serving as a title
        ui.cursive.add_layer(Dialog::around(
                LinearLayout::vertical()
                .child(TextView::new("Title").h_align(HAlign::Center))
                .child(DummyView.fixed_height(1))
                .child(TextView::new("Press q or <quit> to quit")
                       .h_align(HAlign::Center)
                      )
                .child(DummyView.fixed_height(1))
                .child(TextView::new(text))
                .child(TextView::new(text).scrollable())
                .child(TextView::new(text).scrollable())
                .child(TextView::new(text).scrollable())
                .fixed_width(30),
                ).button("Quit", |s| s.quit())
            .h_align(HAlign::Center),
            );

        let controller_tx_clone = ui.controller_tx.clone();
        ui.cursive.add_global_callback(cursive::event::Event::from('q'), move |_c| {
            controller_tx_clone.send(ControllerMessage::Quit()).unwrap();
        });
        let controller_tx_clone = ui.controller_tx.clone();
        ui.cursive.add_global_callback(cursive::event::Event::from('Q'), move |_c| {
            controller_tx_clone.send(ControllerMessage::Quit()).unwrap();
        });
        ui
    }

    /// Step the UI by calling into Cursive's step function, then
    /// processing any UI messages.
    pub fn step(&mut self) -> bool {
        if !self.cursive.is_running() {
            return false;
        }

        // step the ui
        self.cursive.step();

        // Process any pending UI messages
        while let Some(message) = self.ui_rx.try_iter().next() {
            match message {
                UiMessage::Quit() => {
                    self.cursive.quit();
                }
            }
        }

        true
    }
}

pub struct Controller {
    rx: mpsc::Receiver<ControllerMessage>,
    ui: Ui,
}

pub enum ControllerMessage {
    Quit(),
}

impl Controller {
    /// Create a new controller
    pub fn new() -> Result<Controller, String> {
        let (tx, rx) = mpsc::channel::<ControllerMessage>();
        Ok(Controller {
            rx: rx,
            ui: Ui::new(tx.clone()),
        })
    }

    /// Run the controller
    pub fn run(&mut self) {
        while self.ui.step() {
            while let Some(message) = self.rx.try_iter().next() {
                // Handle messages arriving from the UI.
                match message {
                    ControllerMessage::Quit() => {
                        self.ui
                            .cursive.quit();
                    }   
               };
            }
        }
    }
}

fn main() {
    // Launch the controller and UI
    let controller = Controller::new();
    match controller {
        Ok(mut controller) => controller.run(),
        Err(e) => {
            let s = format!("Error creating controller: {}", e);
            eprintln!("{}", s);
        }
    };
}
