use cursive::traits::Resizable;
use cursive::traits::*;
use cursive::views;

use std::io::{Read as _, Write as _};
use std::sync::{Arc, Mutex};

// This example builds a simple TCP server with some parameters and some output.
// It then builds a TUI to control the parameters and display the output.

fn main() {
    let mut siv = cursive::default();

    // Build a shared model
    let model = Arc::new(Mutex::new(ModelData {
        offset: 10,
        logs: Vec::new(),
        cb_sink: siv.cb_sink().clone(),
    }));

    // Start the TCP server in a thread
    start_server(Arc::clone(&model));

    // Build the UI from the model
    siv.add_layer(views::Dialog::around(build_ui(Arc::clone(&model))).button("Quit", |s| s.quit()));

    siv.run();
}

struct ModelData {
    /// The offset will be controlled by the UI and used in the server
    offset: u8,
    /// Logs will be filled by the server and displayed on the UI
    logs: Vec<LogEntry>,
    /// A callback sink is used to control the UI from the server
    /// (eg. force refresh, error popups)
    cb_sink: cursive::CbSink,
}

// Here we use a single mutex, but bigger models might
// prefer individual mutexes for different variables.
type Model = Arc<Mutex<ModelData>>;

#[derive(Clone, Copy)]
struct LogEntry {
    input: u8,
    output: u8,
}

/// Starts serving on a separate thread, and show a popup on error.
fn start_server(model: Model) {
    std::thread::spawn(move || {
        if let Err(err) = serve(Arc::clone(&model)) {
            let model = model.lock().unwrap();
            model
                .cb_sink
                .send(Box::new(move |s: &mut cursive::Cursive| {
                    s.add_layer(
                        views::Dialog::text(format!("{err:?}"))
                            .title("Error in TCP server")
                            .button("Quit", |s| s.quit()),
                    );
                }))
                .unwrap();
        }
    });
}

/// Starts a simple, single-threaded TCP server.
/// Adds a configurable offset to each byte received and sent it back.
fn serve(model: Model) -> std::io::Result<()> {
    // Bind on some local address
    let listener = std::net::TcpListener::bind("localhost:1234")?;

    // Handle each connection sequentially
    for stream in listener.incoming() {
        let stream = stream?;

        // Process each byte according to the current model.
        for byte in (&stream).bytes() {
            let byte = byte?;
            let mut model = model.lock().unwrap();
            let response = byte.wrapping_add(model.offset);
            (&stream).write_all(&[response])?;

            // Save processed jobs
            model.logs.push(LogEntry {
                input: byte,
                output: response,
            });

            // Send a noop to refresh the display
            model
                .cb_sink
                .send(Box::new(cursive::Cursive::noop))
                .unwrap();
        }
    }

    Ok(())
}

/// Build the UI for the given model.
fn build_ui(model: Model) -> impl cursive::view::View {
    // Build the UI in 3 parts, stacked together in a LinearLayout.
    views::LinearLayout::vertical()
        .child(build_selector(Arc::clone(&model)))
        .child(build_tester(Arc::clone(&model)))
        .child(views::DummyView.fixed_height(1))
        .child(build_log_viewer(Arc::clone(&model)))
}

/// Build a view that shows processed jobs from the model.
fn build_log_viewer(model: Model) -> impl cursive::view::View {
    views::Canvas::new(model)
        .with_draw(|model, printer| {
            let model = model.lock().unwrap();
            for (i, &log) in model.logs.iter().enumerate() {
                printer.print(
                    (0, i),
                    &format!(
                        "{:3} '{}'  ->  {:3} {:?}",
                        log.input,
                        readable_char(log.input),
                        log.output,
                        readable_char(log.output),
                    ),
                );
            }
        })
        .with_required_size(|model, _req| {
            let model = model.lock().unwrap();
            cursive::Vec2::new(20, model.logs.len())
        })
        .scrollable()
}

/// Pretty print an ascii u8 if possible.
fn readable_char(byte: u8) -> char {
    if byte.is_ascii_control() {
        '�'
    } else {
        byte as char
    }
}

/// Build a view that can update the model.
fn build_selector(model: Model) -> impl cursive::view::View {
    let offset = model.lock().unwrap().offset;
    views::LinearLayout::horizontal()
        .child(
            views::EditView::new()
                .content(format!("{offset}"))
                .with_name("edit")
                .min_width(5),
        )
        .child(views::DummyView.fixed_width(1))
        .child(views::Button::new("Update", move |s| {
            if let Some(n) = s
                .call_on_name("edit", |edit: &mut views::EditView| edit.get_content())
                .and_then(|content| content.parse().ok())
            {
                model.lock().unwrap().offset = n;
            } else {
                s.add_layer(views::Dialog::info("Could not parse offset as u8"));
            }
        }))
}

/// Build a view that can run test connections.
fn build_tester(model: Model) -> impl cursive::view::View {
    views::LinearLayout::horizontal()
        .child(views::TextView::new("Current value:"))
        .child(views::DummyView.fixed_width(1))
        .child(
            views::Canvas::new(model)
                .with_draw(|model, printer| {
                    printer.print((0, 0), &format!("{}", model.lock().unwrap().offset))
                })
                .with_required_size(|_, _| cursive::Vec2::new(3, 1)),
        )
        .child(views::DummyView.fixed_width(1))
        .child(views::Button::new("Test", |s| {
            if let Err(err) = test_server() {
                s.add_layer(views::Dialog::info(format!("{err:?}")).title("Error running test."));
            }
        }))
}

/// Run a test connection.
fn test_server() -> std::io::Result<()> {
    let mut stream = std::net::TcpStream::connect("localhost:1234")?;
    for &byte in b"cursive123" {
        let mut buf = [0];
        stream.write_all(&[byte])?;
        stream.read_exact(&mut buf)?;
    }
    Ok(())
}
