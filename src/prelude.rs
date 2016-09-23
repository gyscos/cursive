//! Commonly used imports, conveniently grouped.
//!
//! To easily import a bunch of commonly-used definitions, import this bundle:
//!
//! ```
//! use cursive::prelude::*;
//! ```

#[doc(no_inline)]
pub use {Cursive, Printer, With};
#[doc(no_inline)]
pub use event::{Event, Key};
#[doc(no_inline)]
pub use view::{Boxable, Finder, Identifiable, Selector, View};
#[doc(no_inline)]
pub use views::{BoxView, Button, Checkbox, Dialog, DummyView, EditView,
                IdView, KeyEventView, LinearLayout, ListView, Panel,
                ProgressBar, SelectView, SliderView, TextArea, TextView};
#[doc(no_inline)]
pub use vec::Vec2;
#[doc(no_inline)]
pub use menu::MenuTree;
