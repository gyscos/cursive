use logger;
use theme;
use vec::Vec2;
use view::View;
use Printer;

use unicode_width::UnicodeWidthStr;

/// View used for debugging, showing logs.
pub struct DebugView {
    // We'll want to store the formatting (line split)
// ... or 1 line per log?
}

impl DebugView {
    /// Creates a new DebugView.
    pub fn new() -> Self {
        DebugView {}
    }
}

impl View for DebugView {
    fn draw(&self, printer: &Printer) {
        let logs = logger::LOGS.lock().unwrap();
        // Only print the last logs, so skip what doesn't fit
        let skipped = logs.len().saturating_sub(printer.size.y);

        for (i, record) in logs.iter().skip(skipped).enumerate() {
            // TODO: Apply style to message? (Ex: errors in bold?)
            printer.print(
                (0, i),
                &format!(
                    "{} | [     ] {}",
                    record.time.with_timezone(&chrono::Local).format("%T%.3f"),
                    record.message
                ),
            );
            let color = match record.level {
                log::Level::Error => theme::BaseColor::Red,
                log::Level::Warn => theme::BaseColor::Yellow,
                log::Level::Info => theme::BaseColor::Black,
                log::Level::Debug => theme::BaseColor::Green,
                log::Level::Trace => theme::BaseColor::Blue,
            };
            printer.with_color(color.into(), |printer| {
                printer.print((16, i), &format!("{:5}", record.level))
            });
        }
    }

    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        // TODO: read the logs, and compute the required size to print it.
        let logs = logger::LOGS.lock().unwrap();

        let level_width = 8; // Width of "[ERROR] "
        let time_width = 16; // Width of "23:59:59.123 | "
                             // The longest line sets the width
        let w = logs
            .iter()
            .map(|record| record.message.width() + level_width + time_width)
            .max()
            .unwrap_or(1);
        let h = logs.len();

        Vec2::new(w, h)
    }

    fn layout(&mut self, _size: Vec2) {
        // Uh?
    }
}
