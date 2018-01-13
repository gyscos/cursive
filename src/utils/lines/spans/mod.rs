//! Compute lines on multiple spans of text.
//!
//! The input is a list of consecutive text spans.
//!
//! Computed rows will include a list of span segments.
//! Each segment include the source span ID, and start/end byte offsets.
mod lines_iterator;
mod chunk_iterator;
mod segment_merge_iterator;
mod row;
mod prefix;
mod chunk;
mod segment;

#[cfg(test)]
mod tests;

pub use self::lines_iterator::LinesIterator;
pub use self::row::Row;
pub use self::segment::Segment;
