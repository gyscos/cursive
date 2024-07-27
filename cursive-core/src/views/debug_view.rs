use crate::logger;
use crate::style;
use crate::view::View;
use crate::Printer;
use crate::Vec2;

use unicode_width::UnicodeWidthStr;

/// View used for debugging, showing logs.
pub struct DebugView {
    // TODO: wrap log lines if needed, and save the line splits here.
}

impl DebugView {
    /// Creates a new DebugView.
    pub fn new() -> Self {
        DebugView {}
    }
}

impl Default for DebugView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for DebugView {
    fn draw(&self, printer: &Printer) {
        let logs = logger::LOGS.lock().unwrap();
        // Only print the last logs, so skip what doesn't fit
        let skipped = logs.len().saturating_sub(printer.size.y);

        let format =
            time::format_description::parse("[hour]:[minute]:[second].[subsecond digits:3]")
                .unwrap();

        for (i, record) in logs.iter().skip(skipped).enumerate() {
            // TODO: Apply style to message? (Ex: errors in bold?)
            // TODO: customizable time format? (24h/AM-PM)
            let formatted = record
                .time
                .format(&format)
                .unwrap_or_else(|_| String::new());
            printer.print(
                (0, i),
                &format!("{} | [     ] {}", formatted, record.message),
            );
            let color = match record.level {
                log::Level::Error => style::BaseColor::Red.dark(),
                log::Level::Warn => style::BaseColor::Yellow.dark(),
                log::Level::Info => style::BaseColor::Black.light(),
                log::Level::Debug => style::BaseColor::Green.dark(),
                log::Level::Trace => style::BaseColor::Blue.dark(),
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

crate::manual_blueprint!(DebugView, |_, _| { Ok(DebugView::new()) });
