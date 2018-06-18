extern crate ioctl_rs as ioctl;

use libc::{c_ushort, STDOUT_FILENO};
use std::mem;

use vec::Vec2;

#[repr(C)]
struct TermSize {
    row: c_ushort,
    col: c_ushort,
    _x: c_ushort,
    _y: c_ushort,
}

/// Get the size of the terminal.
pub fn terminal_size() -> Vec2 {
    unsafe {
        let mut size: TermSize = mem::zeroed();
        ioctl::ioctl(STDOUT_FILENO, ioctl::TIOCGWINSZ, &mut size as *mut _);
        Vec2::new(size.col as usize, size.row as usize)
    }
}
