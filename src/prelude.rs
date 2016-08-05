//! Commonly used imports, conveniently grouped.
//!
//! To easily import a bunch of commonly-used definitions, import this bundle:
//!
//! ```
//! use cursive::prelude::*;
//! ```

pub use {Cursive, Printer, With};
pub use event::{Event, Key};
pub use view::{Boxable, Identifiable, Selector, View};
pub use views::{BoxView, Button, Checkbox, Dialog, EditView, IdView,
                KeyEventView, LinearLayout, ListView, Panel, ProgressBar,
                SelectView, TextArea, TextView};
pub use vec::Vec2;
pub use menu::MenuTree;
