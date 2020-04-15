use super::chunk::Chunk;
use super::segment::Segment;
use crate::utils::span::SpannedText;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;
use xi_unicode::LineBreakLeafIter;

/// Iterator that returns non-breakable chunks of text.
///
/// Works accross spans of text.
pub struct ChunkIterator<S> {
    /// Input that we want to chunk.
    source: Rc<S>,

    /// ID of the span we are processing.
    current_span: usize,

    /// How much of the current span has been processed already.
    offset: usize,
}

impl<S> ChunkIterator<S> {
    /// Creates a new ChunkIterator on the given styled string.
    pub fn new(source: Rc<S>) -> Self {
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
impl<S> Iterator for ChunkIterator<S>
where
    S: SpannedText,
{
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        // Stop when we processed all spans
        if self.current_span >= self.source.spans().len() {
            return None;
        }

        // Skip empty spans
        if self.source.spans()[self.current_span].as_ref().is_empty() {
            self.current_span += 1;
            return self.next();
        }

        // Current span & associated text
        let mut span = self.source.spans()[self.current_span].as_ref();
        let mut span_text = span.resolve(self.source.source());

        let mut total_width = 0;

        // We'll accumulate segments from spans.
        let mut segments = Vec::new();

        // We'll use an iterator from xi-unicode to detect possible breaks.
        let mut iter = LineBreakLeafIter::new(span_text, self.offset);

        // When we reach the end of a span, xi-unicode returns a break, but it
        // actually depends on the next span. Such breaks are "fake" breaks.
        //
        // So we'll loop until we find a "true" break
        // (a break that doesn't happen an the end of a span).
        // Note that if a break is a "hard" stop, then it is always a "true" break.
        //
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
                    self.source.spans()[self.current_span - 1].as_ref();
                let prev_text = prev_span.resolve(self.source.source());
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
                segments.push(Segment {
                    span_id: self.current_span,
                    start: self.offset,
                    end: pos,
                    width,
                });
            }

            if pos == span_text.len() {
                // If we reached the end of the slice,
                // we need to look at the next span first.
                self.current_span += 1;

                // Skip empty spans
                while let Some(true) =
                    self.source.spans().get(self.current_span).map(|span| {
                        span.as_ref().resolve(self.source.source()).is_empty()
                    })
                {
                    self.current_span += 1;
                }

                if self.current_span >= self.source.spans().len() {
                    // If this was the last chunk, return as is!
                    // Well, make sure we don't end with a newline...
                    let hard_stop = hard_stop || span_text.ends_with('\n');

                    return Some(Chunk {
                        width: total_width,
                        segments,
                        hard_stop,
                        ends_with_space,
                    });
                }

                span = self.source.spans()[self.current_span].as_ref();
                span_text = span.resolve(self.source.source());
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
