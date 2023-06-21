use super::segment::Segment;

/// Non-splittable piece of text.
///
/// It is made of a list of segments of text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chunk {
    /// Total width of this chunk.
    pub width: usize,

    /// This is the segments this chunk contains.
    pub segments: Vec<Segment>,

    /// Hard stops are non-optional line breaks (newlines).
    pub hard_stop: bool,

    /// If a chunk of text ends in a space, it can be compressed a bit.
    ///
    /// (We can omit the space if it would result in a perfect fit.)
    ///
    /// This only matches literally the ' ' byte.
    pub ends_with_space: bool,
}

impl Chunk {
    /// Remove some text from the front.
    ///
    /// We're given the length (number of bytes) and the width.
    pub fn remove_front(&mut self, mut to_remove: ChunkPart) {
        // Remove something from each segment until we've removed enough.
        for segment in &mut self.segments {
            if to_remove.length <= segment.end - segment.start {
                // This segment is bigger than what we need to remove
                // So just trim the prefix and stop there.
                segment.start += to_remove.length;
                segment.width -= to_remove.width;
                self.width -= to_remove.width;
                break;
            } else {
                // This segment is too small, so it'll disappear entirely.
                to_remove.length -= segment.end - segment.start;
                to_remove.width -= segment.width;
                self.width -= segment.width;

                // Empty this segment
                segment.start = segment.end;
                segment.width = 0;
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
            if self.ends_with_space {
                last.end -= 1;
                last.width -= 1;
            }
            last.start == last.end
        };

        if last_empty {
            // self.segments.pop().unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
/// Describes a part of a chunk.
///
/// Includes both length and width to ease some computations.
///
/// This is used to represent how much of a chunk we've already processed.
pub struct ChunkPart {
    pub width: usize,
    pub length: usize,
}
