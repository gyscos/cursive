//! Toolbox to make text layout easier.

mod compact;
mod counter;
mod dot_grid;
#[macro_use]
mod immutify;
pub mod lines;
pub mod markup;
mod reader;
pub mod span;

pub use self::compact::format_compact;
pub use self::counter::Counter;
pub use self::dot_grid::{DotGrid, DotMode};
pub use self::reader::ProgressReader;
