//! Compute lines on multiple spans of text.
//!
//! The input is a list of consecutive text spans.
//!
//! Computed rows will include a list of span segments.
//! Each segment include the source span ID, and start/end byte offsets.

use std::borrow::Cow;
use std::iter::Peekable;
use theme::Style;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use xi_unicode::LineBreakLeafIter;

/// Input to the algorithm
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span<'a> {
    text: Cow<'a, str>,
    style: Style,
}

/// Refers to a part of a span
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// ID of the span this segment refers to
    pub span_id: usize,

    /// Beginning of this segment within the span (included)
    pub start: usize,
    /// End of this segment within the span (excluded)
    pub end: usize,

    /// Width of this segment
    pub width: usize,
}

impl Segment {
    #[cfg(test)]
    fn with_text<'a>(self, text: &'a str) -> SegmentWithText<'a> {
        SegmentWithText { text, seg: self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SegmentWithText<'a> {
    seg: Segment,
    text: &'a str,
}

/// Non-splittable piece of text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Chunk<'a> {
    width: usize,
    segments: Vec<SegmentWithText<'a>>,
    hard_stop: bool,
    ends_with_space: bool,
}

impl<'a> Chunk<'a> {
    /// Remove some text from the front.
    ///
    /// We're given the length (number of bytes) and the width.
    fn remove_front(&mut self, mut to_remove: ChunkPart) {
        // Remove something from each segment until we've removed enough.
        for segment in &mut self.segments {
            if to_remove.length <= segment.seg.end - segment.seg.start {
                // This segment is bigger than what we need to remove
                // So just trim the prefix and stop there.
                segment.seg.start += to_remove.length;
                segment.seg.width -= to_remove.width;
                segment.text = &segment.text[to_remove.length..];
                break;
            } else {
                // This segment is too small, so it'll disapear entirely.
                to_remove.length -= segment.seg.end - segment.seg.start;
                to_remove.width -= segment.seg.width;

                // Empty this segment
                segment.seg.start = segment.seg.end;
                segment.seg.width = 0;
                segment.text = &"";
            }
        }
    }

    /// Remove the last character from this chunk.
    ///
    /// Usually done to remove a trailing space/newline.
    fn remove_last_char(&mut self) {
        // We remove the last char in 2 situations:
        // * Trailing space.
        // * Trailing newline.
        // Only in the first case does this affect width.
        // (Because newlines have 0 width)

        if self.ends_with_space {
            // Only reduce the width if the last char was a space.
            // Otherwise it's a newline, and we don't want to reduce
            // that.
            self.width -= 1;
        }

        // Is the last segment empty after trimming it?
        // If yes, just drop it.
        let last_empty = {
            let last = self.segments.last_mut().unwrap();
            last.seg.end -= 1;
            if self.ends_with_space {
                last.seg.width -= 1;
            }
            last.seg.start == last.seg.end
        };
        if last_empty {
            self.segments.pop().unwrap();
        }
    }
}

/// Iterator that returns non-breakable chunks of text.
///
/// Works accross spans of text.
struct ChunkIterator<'a, 'b>
where
    'a: 'b,
{
    /// Input that we want to split
    spans: &'b [Span<'a>],

    current_span: usize,

    /// How much of the current span has been processed already.
    offset: usize,
}

impl<'a, 'b> ChunkIterator<'a, 'b>
where
    'a: 'b,
{
    fn new(spans: &'b [Span<'a>]) -> Self {
        ChunkIterator {
            spans,
            current_span: 0,
            offset: 0,
        }
    }
}

/// This iterator produces chunks of non-breakable text.
///
/// These chunks may go accross spans (a single word may be broken into more
/// than one span, for instance if parts of it are marked up differently).
impl<'a, 'b> Iterator for ChunkIterator<'a, 'b>
where
    'a: 'b,
{
    type Item = Chunk<'b>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_span >= self.spans.len() {
            return None;
        }

        let mut span: &Span<'a> = &self.spans[self.current_span];

        let mut total_width = 0;

        // We'll use an iterator from xi-unicode to detect possible breaks.
        let mut iter = LineBreakLeafIter::new(&span.text, self.offset);

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
            let (pos, hard_stop) = iter.next(&span.text);

            // When xi-unicode reaches the end of a span, it returns a "fake"
            // break. To know if it's actually a true break, we need to give
            // it the next span. If, given the next span, it returns a break
            // at position 0, then the previous one was a true break.
            // So when pos = 0, we don't really have a new segment, but we
            // can end the current chunk.

            let (width, ends_with_space) = if pos == 0 {
                // If pos = 0, we had a span before.
                let prev_span = &self.spans[self.current_span - 1];
                (0, prev_span.text.ends_with(' '))
            } else {
                // We actually got something.
                // Remember its width, and whether it ends with a space.
                //
                // (When a chunk ends with a space, we may compress it a bit
                // near the end of a row, so this information will be useful
                // later.)
                let text = &span.text[self.offset..pos];

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
                    text: &span.text[self.offset..pos],
                });
            }

            if pos == span.text.len() {
                // If we reached the end of the slice,
                // we need to look at the next span first.
                self.current_span += 1;

                if self.current_span >= self.spans.len() {
                    // If this was the last chunk, return as is!
                    return Some(Chunk {
                        width: total_width,
                        segments,
                        hard_stop,
                        ends_with_space,
                    });
                }

                span = &self.spans[self.current_span];
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

/// A list of segments representing a row of text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// List of segments
    pub segments: Vec<Segment>,
    /// Total width for this row
    pub width: usize,
}

impl Row {
    /// Resolve the row indices into styled spans.
    pub fn resolve<'a: 'b, 'b>(&self, spans: &'b [Span<'a>]) -> Vec<Span<'b>> {
        self.segments
            .iter()
            .map(|seg| {
                let span: &'b Span<'a> = &spans[seg.span_id];
                let text: &'b str = &span.text;
                let text: &'b str = &text[seg.start..seg.end];

                Span {
                    text: Cow::Borrowed(text),
                    style: span.style,
                }
            })
            .collect()
    }
}

/// Generates rows of text in constrainted width.
///
/// Works on spans of text.
pub struct SpanLinesIterator<'a, 'b>
where
    'a: 'b,
{
    iter: Peekable<ChunkIterator<'a, 'b>>,

    /// Available width
    width: usize,

    /// If a chunk wouldn't fit, we had to cut it in pieces.
    /// This is how far in the current chunk we are.
    chunk_offset: ChunkPart,
}

impl<'a, 'b> SpanLinesIterator<'a, 'b>
where
    'a: 'b,
{
    /// Creates a new iterator with the given content and width.
    pub fn new(spans: &'b [Span<'a>], width: usize) -> Self {
        SpanLinesIterator {
            iter: ChunkIterator::new(spans).peekable(),
            width,
            chunk_offset: ChunkPart::default(),
        }
    }
}

/// Result of a fitness test
///
/// Describes how well a chunk fits in the available space.
enum ChunkFitResult {
    /// This chunk can fit as-is
    Fits,

    /// This chunk fits, but it'll be the last one.
    /// Additionally, its last char may need to be removed.
    FitsBarely,

    /// This chunk doesn't fit. Don't even.
    DoesNotFit,
}

/// Look at a chunk, and decide how it could fit.
fn consider_chunk(available: usize, chunk: &Chunk) -> ChunkFitResult {
    if chunk.width <= available {
        // We fits. No question about it.
        if chunk.hard_stop {
            // Still, we have to stop here.
            // And possibly trim a newline.
            ChunkFitResult::FitsBarely
        } else {
            // Nothing special here.
            ChunkFitResult::Fits
        }
    } else if chunk.width == available + 1 {
        // We're just SLIGHTLY too big!
        // Can we just pop something?
        if chunk.ends_with_space {
            // Yay!
            ChunkFitResult::FitsBarely
        } else {
            // Noo(
            ChunkFitResult::DoesNotFit
        }
    } else {
        // Can't bargain with me.
        ChunkFitResult::DoesNotFit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Describes a part of a chunk.
///
/// Includes both length and width to ease some computations.
///
/// This is used to represent how much of a chunk we've already processed.
struct ChunkPart {
    width: usize,
    length: usize,
}

/// Concatenates chunks as long as they fit in the given width.
fn prefix<'a, I>(
    tokens: &mut Peekable<I>, width: usize, offset: &mut ChunkPart
) -> Vec<Chunk<'a>>
where
    I: Iterator<Item = Chunk<'a>>,
{
    let mut available = width;
    let mut chunks = Vec::new();

    // Accumulate chunks until it doesn't fit.
    loop {
        // Look at the next chunk and see if it would fit.
        let result = {
            let next_chunk = match tokens.peek() {
                None => break,
                Some(chunk) => chunk,
            };

            // When considering if the chunk fits, remember that we may
            // already have processed part of it.
            // So (chunk - width) fits available
            // if chunks fits (available + width)
            consider_chunk(available + offset.width, next_chunk)
        };

        match result {
            ChunkFitResult::Fits => {
                // It fits! Add it and move to the next one.
                let mut chunk = tokens.next().unwrap();
                // Remember to strip the prefix, in case we took some earlier.
                chunk.remove_front(*offset);
                // And reset out offset.
                offset.length = 0;
                offset.width = 0;

                available -= chunk.width;
                chunks.push(chunk);
                continue;
            }
            ChunkFitResult::FitsBarely => {
                // That's it, it's the last one and we're off.
                let mut chunk = tokens.next().unwrap();
                chunk.remove_front(*offset);
                offset.length = 0;
                offset.width = 0;

                // We know we need to remove the last character.
                // Because it's either:
                // * A hard stop: there is a newline
                // * A compressed chunk: it ends with a space
                chunk.remove_last_char();
                chunks.push(chunk);
                // No need to update `available`,
                // as we're ending the line anyway.
                break;
            }
            ChunkFitResult::DoesNotFit => {
                break;
            }
        }
    }

    chunks
}

impl<'a, 'b> Iterator for SpanLinesIterator<'a, 'b>
where
    'a: 'b,
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
                    let graphemes = chunk.segments.iter().flat_map(|seg| {
                        let mut offset = seg.seg.start;
                        seg.text.graphemes(true).map(move |g| {
                            let width = g.width();
                            let start = offset;
                            let end = offset + g.len();
                            offset = end;
                            Chunk {
                                width,
                                segments: vec![
                                    SegmentWithText {
                                        text: g,
                                        seg: Segment {
                                            width,
                                            span_id: seg.seg.span_id,
                                            start,
                                            end,
                                        },
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
                        .map(|segment| segment.text.len())
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
                .map(|segment| segment.seg)
                .filter(|segment| segment.start != segment.end),
        ).collect();

        // TODO: merge consecutive segments of the same span

        Some(Row { segments, width })
    }
}

struct SegmentMergeIterator<I> {
    current: Option<Segment>,
    inner: I,
}

impl<I> SegmentMergeIterator<I> {
    fn new(inner: I) -> Self {
        SegmentMergeIterator {
            inner,
            current: None,
        }
    }
}

impl<I> Iterator for SegmentMergeIterator<I>
where
    I: Iterator<Item = Segment>,
{
    type Item = Segment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_none() {
            self.current = self.inner.next();
            if self.current.is_none() {
                return None;
            }
        }

        loop {
            match self.inner.next() {
                None => return self.current.take(),
                Some(next) => {
                    if next.span_id == self.current.unwrap().span_id {
                        let current = self.current.as_mut().unwrap();
                        current.end = next.end;
                        current.width += next.width;
                    } else {
                        let current = self.current.take();
                        self.current = Some(next);
                        return current;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> Vec<Span<'static>> {
        vec![
            Span {
                text: Cow::Borrowed("A beautiful "),
                style: Style::none(),
            },
            Span {
                text: Cow::Borrowed("boat"),
                style: Style::none(),
            },
            Span {
                text: Cow::Borrowed(" isn't it?\nYes indeed, my "),
                style: Style::none(),
            },
            Span {
                text: Cow::Borrowed("Super"),
                style: Style::none(),
            },
            Span {
                text: Cow::Borrowed("Captain !"),
                style: Style::none(),
            },
        ]
    }

    #[test]
    fn test_lines_iter() {
        let input = input();

        let iter = SpanLinesIterator::new(&input, 16);
        let rows: Vec<Row> = iter.collect();
        let spans: Vec<_> =
            rows.iter().map(|row| row.resolve(&input)).collect();

        assert_eq!(
            &spans[..],
            &[
                vec![
                    Span {
                        text: Cow::Borrowed("A beautiful "),
                        style: Style::none(),
                    },
                    Span {
                        text: Cow::Borrowed("boat"),
                        style: Style::none(),
                    }
                ],
                vec![
                    Span {
                        text: Cow::Borrowed("isn\'t it?"),
                        style: Style::none(),
                    }
                ],
                vec![
                    Span {
                        text: Cow::Borrowed("Yes indeed, my "),
                        style: Style::none(),
                    }
                ],
                vec![
                    Span {
                        text: Cow::Borrowed("Super"),
                        style: Style::none(),
                    },
                    Span {
                        text: Cow::Borrowed("Captain !"),
                        style: Style::none(),
                    }
                ]
            ]
        );

        assert_eq!(
            &rows[..],
            &[
                Row {
                    segments: vec![
                        Segment {
                            span_id: 0,
                            start: 0,
                            end: 12,
                            width: 12,
                        },
                        Segment {
                            span_id: 1,
                            start: 0,
                            end: 4,
                            width: 4,
                        },
                    ],
                    width: 16,
                },
                Row {
                    segments: vec![
                        Segment {
                            span_id: 2,
                            start: 1,
                            end: 10,
                            width: 9,
                        },
                    ],
                    width: 9,
                },
                Row {
                    segments: vec![
                        Segment {
                            span_id: 2,
                            start: 11,
                            end: 26,
                            width: 15,
                        },
                    ],
                    width: 15,
                },
                Row {
                    segments: vec![
                        Segment {
                            span_id: 3,
                            start: 0,
                            end: 5,
                            width: 5,
                        },
                        Segment {
                            span_id: 4,
                            start: 0,
                            end: 9,
                            width: 9,
                        },
                    ],
                    width: 14,
                }
            ]
        );
    }

    #[test]
    fn test_chunk_iter() {
        let input = input();

        let iter = ChunkIterator::new(&input);
        let chunks: Vec<Chunk> = iter.collect();

        assert_eq!(
            &chunks[..],
            &[
                Chunk {
                    width: 2,
                    segments: vec![
                        Segment {
                            span_id: 0,
                            start: 0,
                            end: 2,
                            width: 2,
                        }.with_text("A "),
                    ],
                    hard_stop: false,
                    ends_with_space: true,
                },
                Chunk {
                    width: 10,
                    segments: vec![
                        Segment {
                            span_id: 0,
                            start: 2,
                            end: 12,
                            width: 10,
                        }.with_text("beautiful "),
                    ],
                    hard_stop: false,
                    ends_with_space: true,
                },
                Chunk {
                    width: 5,
                    segments: vec![
                        Segment {
                            span_id: 1,
                            start: 0,
                            end: 4,
                            width: 4,
                        }.with_text("boat"),
                        Segment {
                            span_id: 2,
                            start: 0,
                            end: 1,
                            width: 1,
                        }.with_text(" "),
                    ],
                    hard_stop: false,
                    ends_with_space: true,
                },
                Chunk {
                    width: 6,
                    segments: vec![
                        // "isn't "
                        Segment {
                            span_id: 2,
                            start: 1,
                            end: 7,
                            width: 6,
                        }.with_text("isn't "),
                    ],
                    hard_stop: false,
                    ends_with_space: true,
                },
                Chunk {
                    width: 3,
                    segments: vec![
                        // "it?\n"
                        Segment {
                            span_id: 2,
                            start: 7,
                            end: 11,
                            width: 3,
                        }.with_text("it?\n"),
                    ],
                    hard_stop: true,
                    ends_with_space: false,
                },
                Chunk {
                    width: 4,
                    segments: vec![
                        // "Yes "
                        Segment {
                            span_id: 2,
                            start: 11,
                            end: 15,
                            width: 4,
                        }.with_text("Yes "),
                    ],
                    hard_stop: false,
                    ends_with_space: true,
                },
                Chunk {
                    width: 8,
                    segments: vec![
                        // "indeed, "
                        Segment {
                            span_id: 2,
                            start: 15,
                            end: 23,
                            width: 8,
                        }.with_text("indeed, "),
                    ],
                    hard_stop: false,
                    ends_with_space: true,
                },
                Chunk {
                    width: 3,
                    segments: vec![
                        // "my "
                        Segment {
                            span_id: 2,
                            start: 23,
                            end: 26,
                            width: 3,
                        }.with_text("my "),
                    ],
                    hard_stop: false,
                    ends_with_space: true,
                },
                Chunk {
                    width: 14,
                    segments: vec![
                        // "Super"
                        Segment {
                            span_id: 3,
                            start: 0,
                            end: 5,
                            width: 5,
                        }.with_text("Super"),
                        // "Captain !"
                        Segment {
                            span_id: 4,
                            start: 0,
                            end: 9,
                            width: 9,
                        }.with_text("Captain !"),
                    ],
                    hard_stop: false,
                    ends_with_space: false,
                }
            ]
        );
    }
}
