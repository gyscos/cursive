use super::segment::SegmentWithText;

/// Non-splittable piece of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub width: usize,
    pub segments: Vec<SegmentWithText<'a>>,
    pub hard_stop: bool,
    pub ends_with_space: bool,
}

impl<'a> Chunk<'a> {
    /// Remove some text from the front.
    ///
    /// We're given the length (number of bytes) and the width.
    pub fn remove_front(&mut self, mut to_remove: ChunkPart) {
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
                segment.text = "";
            }
        }
    }

    /// Remove the last character from this chunk.
    ///
    /// Usually done to remove a trailing space/newline.
    pub fn remove_last_char(&mut self) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Describes a part of a chunk.
///
/// Includes both length and width to ease some computations.
///
/// This is used to represent how much of a chunk we've already processed.
pub struct ChunkPart {
    pub width: usize,
    pub length: usize,
}
