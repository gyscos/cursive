//! Compute lines on multiple spans of text.
//!
//! The input is a list of consecutive text spans.
//!
//! Computed rows will include a list of span segments.
//! Each segment include the source span ID, and start/end byte offsets.
mod chunk;
mod chunk_iterator;
mod lines_iterator;
mod prefix;
mod row;
mod segment;
mod segment_merge_iterator;

#[cfg(test)]
mod tests;

pub use self::lines_iterator::LinesIterator;
pub use self::row::Row;
pub use self::segment::Segment;
