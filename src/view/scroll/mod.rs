//! Core mechanisms to implement scrolling.
//!
//! This module defines [`ScrollCore`](crate::view::scroll::ScrollCore) and related traits.
//!
//! [`ScrollView`](crate::views::ScrollView) may be an easier way to add scrolling to an existing view.

mod base;
mod core;
mod traits;

pub use self::base::ScrollBase;
pub use self::core::ScrollCore;
pub use self::traits::{InnerLayout, InnerOnEvent, InnerRequiredSize};

/// Defines the scrolling behaviour on content or size change
#[derive(Debug)]
pub enum ScrollStrategy {
    /// Keeps the same row number
    KeepRow,
    /// Sticks to the top.
    StickToTop,
    /// Sticks to the bottom of the view.
    StickToBottom,
}

impl Default for ScrollStrategy {
    fn default() -> Self {
        ScrollStrategy::KeepRow
    }
}
