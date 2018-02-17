use super::chunk::{Chunk, ChunkPart};
use super::chunk_iterator::ChunkIterator;
use super::prefix::prefix;
use super::row::Row;
use super::segment::{Segment, SegmentWithText};
use super::segment_merge_iterator::SegmentMergeIterator;
use std::iter::Peekable;
use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use utils::span::SpannedText;

/// Generates rows of text in constrainted width.
///
/// Works on spans of text.
pub struct LinesIterator<S>
where
    S: SpannedText,
{
    iter: Peekable<ChunkIterator<S>>,
    source: Rc<S>,

    /// Available width
    width: usize,

    /// If a chunk wouldn't fit, we had to cut it in pieces.
    /// This is how far in the current chunk we are.
    chunk_offset: ChunkPart,
}

impl<S> LinesIterator<S>
where
    S: SpannedText,
{
    /// Creates a new iterator with the given content and width.
    pub fn new(source: S, width: usize) -> Self {
        let source = Rc::new(source);
        let chunk_source = source.clone();
        LinesIterator {
            iter: ChunkIterator::new(chunk_source).peekable(),
            source,
            width,
            chunk_offset: ChunkPart::default(),
        }
    }
}

impl<S> Iterator for LinesIterator<S>
where
    S: SpannedText,
{
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        // Let's build a beautiful row.

        let mut chunks =
            prefix(&mut self.iter, self.width, &mut self.chunk_offset);

        if chunks.is_empty() {
            // Desperate action to make something fit:
            // Look at the current chunk. We'll try to return a part of it.
            // So now, consider each individual grapheme as a valid chunk.
            // Note: it may not be the first time we try to fit this chunk,
            // so remember to trim the offset we may have stored.
            match self.iter.peek() {
                None => return None,
                Some(chunk) => {
                    let mut chunk = chunk.clone();
                    chunk.remove_front(self.chunk_offset);

                    // Try to fit part of it?
                    let source = self.source.as_ref();
                    let graphemes = chunk.segments.iter().flat_map(move |seg| {
                        let mut offset = seg.start;

                        let text = seg.resolve_plain(source);

                        text.graphemes(true).map(move |g| {
                            let width = g.width();
                            let start = offset;
                            let end = offset + g.len();
                            offset = end;
                            Chunk {
                                width,
                                segments: vec![
                                    Segment {
                                        width,
                                        span_id: seg.span_id,
                                        start,
                                        end,
                                    },
                                ],
                                hard_stop: false,
                                ends_with_space: false,
                            }
                        })
                    });
                    chunks = prefix(
                        &mut graphemes.peekable(),
                        self.width,
                        &mut ChunkPart::default(),
                    );

                    if chunks.is_empty() {
                        // Seriously? After everything we did for you?
                        return None;
                    }

                    // We are going to return a part of a chunk.
                    // So remember what we selected,
                    // so we can skip it next time.
                    let width: usize =
                        chunks.iter().map(|chunk| chunk.width).sum();
                    let length: usize = chunks
                        .iter()
                        .flat_map(|chunk| chunk.segments.iter())
                        .map(|segment| segment.end - segment.start)
                        .sum();

                    self.chunk_offset.width += width;
                    self.chunk_offset.length += length;
                }
            }
        }

        let width = chunks.iter().map(|c| c.width).sum();
        assert!(width <= self.width);

        // Concatenate all segments
        let segments = SegmentMergeIterator::new(
            chunks
                .into_iter()
                .flat_map(|chunk| chunk.segments)
                .filter(|segment| segment.start != segment.end),
        ).collect();

        // TODO: merge consecutive segments of the same span

        Some(Row { segments, width })
    }
}
