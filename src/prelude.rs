//! Commonly used imports, conveniently grouped.
//!
//! To easily import a bunch of commonly-used definitions, import this bundle:
//!
//! ```
//! use cursive::prelude::*;
//! ```

pub use {Cursive, Printer, With};
pub use event::{Event, Key};
pub use view::{BoxView, Button, Checkbox, Dialog, EditView, FullView, IdView,
               Identifiable, KeyEventView, LinearLayout, ListView,
               ProgressBar, SelectView, Selector, TextView, View};
pub use vec::Vec2;
pub use menu::MenuTree;
