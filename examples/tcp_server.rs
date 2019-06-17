use cursive::traits::*;
use cursive::views;

use std::io::Read as _;
use std::io::Write as _;
use std::sync::{Arc, Mutex};

// This example builds a simple TCP server with some parameters and some output.
// It then builds a TUI to control the parameters and display the output.

fn main() {
    let mut siv = cursive::Cursive::default();

    // Build a shared model
    let model = Arc::new(Mutex::new(Model {
        offset: 0,
        logs: Vec::new(),
        cb_sink: siv.cb_sink().clone(),
    }));

    // Start the TCP server in a thread
    start_server(Arc::clone(&model));

    // Build the UI from the model
    siv.add_layer(
        views::Dialog::around(build_ui(Arc::clone(&model)))
            .button("Quit", |s| s.quit()),
    );

    siv.run();
}

struct Model {
    offset: u8,
    logs: Vec<(u8, u8)>,
    cb_sink: cursive::CbSink,
}

fn start_server(model: Arc<Mutex<Model>>) {
    std::thread::spawn(move || {
        if let Err(err) = serve(Arc::clone(&model)) {
            model
                .lock()
                .unwrap()
                .cb_sink
                .send(Box::new(move |s: &mut cursive::Cursive| {
                    s.add_layer(
                        views::Dialog::text(&format!("{:?}", err))
                            .title("Error in TCP server")
                            .button("Quit", |s| s.quit()),
                    );
                }))
                .unwrap();
        }
    });
}

fn serve(model: Arc<Mutex<Model>>) -> std::io::Result<()> {
    let listener = std::net::TcpListener::bind("localhost:1234")?;

    for stream in listener.incoming() {
        let stream = stream?;

        for byte in (&stream).bytes() {
            let byte = byte?;
            let mut model = model.lock().unwrap();
            let response = byte.wrapping_add(model.offset);
            model.logs.push((byte, response));
            (&stream).write_all(&[response])?;
            model
                .cb_sink
                .send(Box::new(cursive::Cursive::noop))
                .unwrap();
        }
    }

    Ok(())
}

fn readable_char(byte: u8) -> char {
    if byte.is_ascii_control() {
        '�'
    } else {
        byte as char
    }
}

fn build_log_viewer(model: Arc<Mutex<Model>>) -> impl cursive::view::View {
    views::Canvas::new(model)
        .with_draw(|model, printer| {
            let model = model.lock().unwrap();
            for (i, &(byte, answer)) in model.logs.iter().enumerate() {
                printer.print(
                    (0, i),
                    &format!(
                        "{:3} '{}' -> {:3} '{}'",
                        byte,
                        readable_char(byte),
                        answer,
                        readable_char(answer),
                    ),
                );
            }
        })
        .with_required_size(|model, _req| {
            let model = model.lock().unwrap();
            cursive::Vec2::new(10, model.logs.len())
        })
}

fn build_selector(model: Arc<Mutex<Model>>) -> impl cursive::view::View {
    views::LinearLayout::horizontal()
        .child(
            views::EditView::new()
                .content("0")
                .with_id("edit")
                .min_width(5),
        )
        .child(views::DummyView.fixed_width(1))
        .child(views::Button::new("Update", move |s| {
            if let Some(n) = s
                .call_on_id("edit", |edit: &mut views::EditView| {
                    edit.get_content()
                })
                .and_then(|content| content.parse().ok())
            {
                model.lock().unwrap().offset = n;
            } else {
                s.add_layer(views::Dialog::info(
                    "Could not parse offset as u8",
                ));
            }
        }))
        .child(views::DummyView.fixed_width(1))
        .child(views::Button::new("Test", |s| {
            if let Err(err) = test_server() {
                s.add_layer(
                    views::Dialog::info(&format!("{:?}", err))
                        .title("Error running test."),
                );
            }
        }))
}

fn test_server() -> std::io::Result<()> {
    let mut stream = std::net::TcpStream::connect("localhost:1234")?;
    for &byte in &[1, 2, 3, b'a', b'c', b'd'] {
        let mut buf = [0];
        stream.write_all(&[byte])?;
        stream.read_exact(&mut buf)?;
    }
    Ok(())
}

fn build_ui(model: Arc<Mutex<Model>>) -> impl cursive::view::View {
    views::LinearLayout::vertical()
        .child(build_selector(Arc::clone(&model)))
        .child(build_log_viewer(Arc::clone(&model)))
}
