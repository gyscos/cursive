//! Commonly used traits bundled for easy import.
//!
//! This module provides an easy way to import some traits.
//!
//! # Examples
//!
//! ```
//! use cursive::traits::*;
//! ```

#[doc(no_inline)]
#[allow(deprecated)]
pub use crate::view::{
    Boxable, Finder, Identifiable, Nameable, Resizable, Scrollable, View,
};

#[doc(no_inline)]
pub use crate::With;
