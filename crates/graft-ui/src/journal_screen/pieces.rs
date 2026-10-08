//! A journal row cut into pieces: glyph, source, age, words, mark, and the line under it.

use crate::keymap::Keymap;

use super::{
    AGE, COMMANDS_HEADING, CURSOR, Columns, GLYPH, Ink, JournalRowView, MARK, Piece, RowKind,
    RowMark, SOURCE, SanitizedText, UnicodeWidthStr, WORDS_BEFORE_GIT, cut_at_word, moved_heading,
    pad,
};

/// A row's own line: cursor, glyph, source word, age, words, and a `●`
/// row's mark or a `$` row's `git:` column.
pub(super) fn row_pieces(
    row: &JournalRowView,
    selected: bool,
    columns: Columns,
    width: usize,
    keys: &Keymap,
) -> Vec<Piece> {
    let cursor = if selected { "▸ " } else { "  " };
    let mut pieces = vec![Piece {
        x: 0,
        text: cursor.to_owned(),
        ink: Ink::Text,
    }];
    let words_x = columns.words_x();
    match row.kind {
        RowKind::Graft | RowKind::Outside => {
            let (glyph, source) = if row.kind == RowKind::Graft {
                ("●", "Graft")
            } else {
                ("$", "outside Graft")
            };
            let source_ink = if row.kind == RowKind::Graft {
                Ink::Text
            } else {
                Ink::Muted
            };
            pieces.push(Piece {
                x: CURSOR,
                text: glyph.to_owned(),
                ink: source_ink,
            });
            let mut x = CURSOR + GLYPH;
            if columns.source {
                pieces.push(Piece {
                    x,
                    text: source.to_owned(),
                    ink: source_ink,
                });
                x += SOURCE;
            }
            pieces.push(Piece {
                x,
                text: row.age.as_str().to_owned(),
                ink: Ink::Text,
            });
            pieces.extend(words_pieces(row, columns, width, words_x, keys));
        }
        RowKind::Lost | RowKind::LostEdit => {
            // `└ ✗ lost` under the event's glyph and source, the words in
            // the age's column: the sha7 stands where the time does.
            let lead_x = CURSOR + GLYPH;
            pieces.push(Piece {
                x: lead_x,
                text: "└ ✗ lost".to_owned(),
                ink: Ink::Warning,
            });
            let x = CURSOR + GLYPH + if columns.source { SOURCE } else { AGE };
            pieces.push(Piece {
                x,
                text: cut_at_word(row.description.as_str(), width.saturating_sub(x + 1)),
                ink: Ink::Warning,
            });
        }
    }
    pieces
}

/// The words of a `●` or `$` row, and what stands at their right.
pub(super) fn words_pieces(
    row: &JournalRowView,
    columns: Columns,
    width: usize,
    words_x: usize,
    keys: &Keymap,
) -> Vec<Piece> {
    let room = width.saturating_sub(words_x);
    match row.kind {
        RowKind::Graft => graft_pieces(row, room, width, words_x, keys),
        RowKind::Outside => match (&row.git_subject, columns.git) {
            (Some(subject), true) => {
                let words_room = room.saturating_sub(1).min(WORDS_BEFORE_GIT);
                let git_x = words_x + words_room + 1;
                vec![
                    Piece {
                        x: words_x,
                        text: cut_at_word(row.description.as_str(), words_room),
                        ink: Ink::Text,
                    },
                    Piece {
                        x: git_x,
                        text: git_words(subject),
                        ink: Ink::Muted,
                    },
                ]
            }
            _ => vec![Piece {
                x: words_x,
                text: cut_at_word(row.description.as_str(), room.saturating_sub(1)),
                ink: Ink::Text,
            }],
        },
        RowKind::Lost | RowKind::LostEdit => Vec::new(),
    }
}

/// A `●` row's description and summary, and its mark at the right edge.
pub(super) fn graft_pieces(
    row: &JournalRowView,
    room: usize,
    width: usize,
    words_x: usize,
    keys: &Keymap,
) -> Vec<Piece> {
    // The summaries line up under each other, which is what makes
    // the list scannable; the description's column is capped so a
    // long one does not push every summary off a wide screen.
    let room = room.saturating_sub(MARK);
    let summary = row.summary.as_str();
    let words = if summary.is_empty() {
        row.description.as_str().to_owned()
    } else {
        // The verdict — the summary's first clause — is what must
        // survive a short row (m4 acceptance review M2: `in progress —
        // c continue  a abort`, `not undoable — origin has it`): where
        // it would not fit, that row's description gives way, cut at a
        // word, down to a floor it keeps. The rest of the summary (`2
        // refs · the index …`) is what a short row loses first, so every
        // other row keeps the shared column. (+ 2: the space before the
        // summary, and the `…` a cut after the verdict ends with.)
        const FLOOR: usize = 12;
        let verdict = summary.split(" · ").next().unwrap_or(summary).width();
        let column = room
            .saturating_sub(30)
            .min(44)
            .min(room.saturating_sub(verdict + 2))
            .max(FLOOR);
        format!(
            "{} {summary}",
            pad(&cut_at_word(row.description.as_str(), column), column),
        )
    };
    let mut pieces = vec![Piece {
        x: words_x,
        text: cut_at_word(&words, room),
        ink: Ink::Text,
    }];
    let mark = row.mark.label(keys);
    if !mark.is_empty() {
        pieces.push(Piece {
            x: width.saturating_sub(mark.width() + 1),
            text: mark,
            ink: if row.mark == RowMark::Interrupted {
                Ink::Warning
            } else {
                Ink::Text
            },
        });
    }
    pieces
}

/// `git: "reset: moving to HEAD~1"`.
pub(super) fn git_words(subject: &SanitizedText) -> String {
    format!("git: \"{}\"", subject.as_str())
}

/// The lines under a row: a selected `$` row's git subject where its column
/// is not drawn, and an expanded `●` row's reason, the refs its fetch
/// moved, and its Command Log.
pub(super) fn under_pieces(
    row: &JournalRowView,
    selected: bool,
    columns: Columns,
    width: usize,
    keys: &Keymap,
) -> Vec<Vec<Piece>> {
    let x = columns.words_x();
    let room = width.saturating_sub(x + 1);
    let one = |text: String, ink: Ink| {
        vec![Piece {
            x,
            text: cut_at_word(&text, room),
            ink,
        }]
    };
    let mut lines = Vec::new();
    if row.kind == RowKind::Outside
        && selected
        && !columns.git
        && let Some(subject) = &row.git_subject
    {
        lines.push(one(git_words(subject), Ink::Muted));
    }
    if row.expanded {
        if let Some(failure) = &row.failure {
            lines.push(one(failure.as_str().to_owned(), Ink::Warning));
        }
        if !row.moved.is_empty() {
            lines.push(one(moved_heading(keys), Ink::Muted));
        }
        for moved in &row.moved {
            lines.push(one(format!("┆ {}", moved.as_str()), Ink::Muted));
        }
        if !row.commands.is_empty() {
            lines.push(one(COMMANDS_HEADING.to_owned(), Ink::Muted));
        }
        for command in &row.commands {
            lines.push(one(format!("┆ {}", command.as_str()), Ink::Muted));
        }
    }
    lines
}
