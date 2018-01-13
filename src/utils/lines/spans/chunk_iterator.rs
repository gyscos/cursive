use super::chunk::Chunk;
use super::segment::{Segment, SegmentWithText};
use unicode_width::UnicodeWidthStr;
use utils::span::SpannedString;
use xi_unicode::LineBreakLeafIter;

/// Iterator that returns non-breakable chunks of text.
///
/// Works accross spans of text.
pub struct ChunkIterator<'a, T>
where
    T: 'a,
{
    /// Input that we want to chunk.
    source: &'a SpannedString<T>,

    /// ID of the span we are processing.
    current_span: usize,

    /// How much of the current span has been processed already.
    offset: usize,
}

impl<'a, T> ChunkIterator<'a, T>
where
    T: 'a,
{
    /// Creates a new ChunkIterator on the given styled string.
    pub fn new(source: &'a SpannedString<T>) -> Self {
        ChunkIterator {
            source,
            current_span: 0,
            offset: 0,
        }
    }
}

/// This iterator produces chunks of non-breakable text.
///
/// These chunks may go accross spans (a single word may be broken into more
/// than one span, for instance if parts of it are marked up differently).
impl<'a, T> Iterator for ChunkIterator<'a, T>
where
    T: 'a,
{
    type Item = Chunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_span >= self.source.spans_raw().len() {
            return None;
        }

        // Protect agains empty spans
        if self.source.spans_raw()[self.current_span].is_empty() {
            self.current_span += 1;
            return self.next();
        }

        let mut span = &self.source.spans_raw()[self.current_span];
        let mut span_text = span.content.resolve(self.source.source());

        let mut total_width = 0;

        // We'll use an iterator from xi-unicode to detect possible breaks.
        let text = span.content.resolve(self.source.source());
        let mut iter = LineBreakLeafIter::new(text, self.offset);

        // We'll accumulate segments from spans.
        let mut segments = Vec::new();

        // When we reach the end of a span, xi-unicode returns a break, but it
        // actually depends on the next span. Such breaks are "fake" breaks.
        // So we'll loop until we find a "true" break
        // (a break that doesn't happen an the end of a span).
        // Most of the time, it will happen on the first iteration.
        loop {
            // Look at next possible break
            // `hard_stop = true` means that the break is non-optional,
            // like after a `\n`.
            let (pos, hard_stop) = iter.next(span_text);

            // When xi-unicode reaches the end of a span, it returns a "fake"
            // break. To know if it's actually a true break, we need to give
            // it the next span. If, given the next span, it returns a break
            // at position 0, then the previous one was a true break.
            // So when pos = 0, we don't really have a new segment, but we
            // can end the current chunk.

            let (width, ends_with_space) = if pos == 0 {
                // If pos = 0, we had a span before.
                let prev_span =
                    &self.source.spans_raw()[self.current_span - 1];
                let prev_text =
                    prev_span.content.resolve(self.source.source());
                (0, prev_text.ends_with(' '))
            } else {
                // We actually got something.
                // Remember its width, and whether it ends with a space.
                //
                // (When a chunk ends with a space, we may compress it a bit
                // near the end of a row, so this information will be useful
                // later.)
                let text = &span_text[self.offset..pos];

                (text.width(), text.ends_with(' '))
            };

            if pos != 0 {
                // If pos != 0, we got an actual segment of a span.
                total_width += width;
                segments.push(SegmentWithText {
                    seg: Segment {
                        span_id: self.current_span,
                        start: self.offset,
                        end: pos,
                        width,
                    },
                    text: &span_text[self.offset..pos],
                });
            }

            if pos == span_text.len() {
                // If we reached the end of the slice,
                // we need to look at the next span first.
                self.current_span += 1;

                // Skip empty spans
                while let Some(true) = self.source
                    .spans_raw()
                    .get(self.current_span)
                    .map(|span| {
                        span.content.resolve(self.source.source()).is_empty()
                    }) {
                    self.current_span += 1;
                }

                if self.current_span >= self.source.spans_raw().len() {
                    // If this was the last chunk, return as is!
                    // Well, make sure we don't end with a newline...
                    let text = span.content.resolve(self.source.source());
                    let hard_stop = hard_stop || text.ends_with('\n');

                    return Some(Chunk {
                        width: total_width,
                        segments,
                        hard_stop,
                        ends_with_space,
                    });
                }

                span = &self.source.spans_raw()[self.current_span];
                span_text = span.content.resolve(self.source.source());
                self.offset = 0;
                continue;
            }

            // Remember where we are.
            self.offset = pos;

            // We found a valid stop, return the current chunk.
            return Some(Chunk {
                width: total_width,
                segments,
                hard_stop,
                ends_with_space,
            });
        }
    }
}
