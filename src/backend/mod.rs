use event;
use theme;

// Module is not named `ncurses` to avoir naming conflict
mod curses;

pub use self::curses::NcursesBackend;

pub trait Backend {

    fn init();
    fn finish();

    fn clear();
    fn refresh();

    fn has_colors() -> bool;

    fn init_color_style(style: theme::ColorStyle,
                        foreground: &theme::Color,
                        background: &theme::Color);

    fn print_at((usize, usize), &str);

    fn poll_event() -> event::Event;
    fn set_refresh_rate(fps: u32);
    fn screen_size() -> (usize, usize);

    fn with_color<F: FnOnce()>(color: theme::ColorStyle, f: F);
    fn with_effect<F: FnOnce()>(effect: theme::Effect, f: F);
}
