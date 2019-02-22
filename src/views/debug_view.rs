use logger;
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

        for (i, &(level, ref text)) in logs.iter().skip(skipped).enumerate() {
            printer.print((0, i), &format!("[{}] {}", level, text));
        }
    }

    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        // TODO: read the logs, and compute the required size to print it.
        let logs = logger::LOGS.lock().unwrap();

        // The longest line sets the width
        let w = logs
            .iter()
            .map(|&(_, ref content)| content.width() + "[ERROR] ".width())
            .max()
            .unwrap_or(1);
        let h = logs.len();

        Vec2::new(w, h)
    }

    fn layout(&mut self, _size: Vec2) {
        // Uh?
    }
}
