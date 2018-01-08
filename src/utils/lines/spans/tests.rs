use theme::Style;
use super::Span;
use super::chunk::Chunk;
use super::row::Row;
use super::chunk_iterator::ChunkIterator;
use super::segment::Segment;

use super::SpanLinesIterator;

use std::borrow::Cow;

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
    let spans: Vec<_> = rows.iter().map(|row| row.resolve(&input)).collect();

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
                },
            ],
            vec![
                Span {
                    text: Cow::Borrowed("isn\'t it?"),
                    style: Style::none(),
                },
            ],
            vec![
                Span {
                    text: Cow::Borrowed("Yes indeed, my "),
                    style: Style::none(),
                },
            ],
            vec![
                Span {
                    text: Cow::Borrowed("Super"),
                    style: Style::none(),
                },
                Span {
                    text: Cow::Borrowed("Captain !"),
                    style: Style::none(),
                },
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
