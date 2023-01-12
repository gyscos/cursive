use super::chunk::{Chunk, ChunkPart};
use std::iter::Peekable;

/// Concatenates chunks as long as they fit in the given width.
pub fn prefix<I>(tokens: &mut Peekable<I>, width: usize, offset: &mut ChunkPart) -> Vec<Chunk>
where
    I: Iterator<Item = Chunk>,
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
