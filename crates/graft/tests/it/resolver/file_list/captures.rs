//! The file list's captures the user judges by eye (m6-conflicts/13),
//! redrawn for the list column (ease-pass/11). Split from `file_list.rs`
//! to keep it under the size ratchet.

use super::*;

/// A done file opened read-only: the list with `▸` on the done row, its
/// identity row, its plain lines, and the done bar. The merge was begun in
/// the shell, so nothing kept `gone.txt`'s stages: no `d` on the row or the
/// bar (m6-conflicts/28).
#[test]
fn done_open_capture() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    let state = done_open(&app, &state, "gone.txt");
    // The status line (`✓ resolved gone.txt (ours)`) holds the bar's row
    // until the next key; `PgUp` at the top scrolls nowhere and gives it
    // back (`k` steps files since m6-conflicts/17).
    let state = app.key(&state, KeyCode::PageUp);
    for (columns, rows) in [(120u16, 40u16), (200, 50), (187, 50)] {
        let frame = crate::captures::frame(&state, columns, rows);
        let body = frame.join("\n");
        assert!(body.contains("▸ gone.txt · done ✓"), "{body}");
        assert!(!body.contains("d starts it over"), "{body}");
        // `o` kept ours: the file on disk is main's plan.
        assert!(body.contains("our plan"), "{body}");
        // A done file has no sides left: one document at every width, never
        // the three panes, whose `theirs · feature` would put the file on
        // disk under the side that deleted it.
        assert!(!body.contains("theirs · feature"), "{body}");
        assert!(
            frame
                .last()
                .is_some_and(|bar| bar.trim() == "e edit  esc close"),
            "{body}"
        );
        if columns == 120 {
            let list = [list_row(&frame, "gone.txt"), list_row(&frame, "notes.txt")];
            assert_eq!(list, ["▏✓ gone.txt done", "? notes.txt 2 open"], "{body}");
        }
        crate::captures::check_shot(
            &format!("m6/fix-file-list-done-open-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
    drop(state);
    app.close();
    drop(scratch);
}

/// A click acknowledges a finished write's status line the way a key does
/// (m6-conflicts/42): after the list click that opens the done file, the
/// bottom row is the bar, not `✓ resolved gone.txt (ours)`.
#[test]
fn a_click_clears_the_done_status_like_a_key() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    assert!(
        matches!(state.status, graft::update::Status::Done { .. }),
        "{:?}",
        state.status
    );
    let state = done_open(&app, &state, "gone.txt");
    let frame = crate::captures::frame(&state, 120, 40);
    assert_eq!(
        frame.last().map(|bar| bar.trim()),
        Some("e edit  esc close"),
        "{}",
        frame.join("\n")
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// ADR-0043's list at every width the ticket names, and the user's own
/// 187×50.
#[test]
fn file_list_capture() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    for (columns, rows) in [(120u16, 40u16), (160, 48), (80, 24), (200, 50), (187, 50)] {
        let frame = crate::captures::frame(&state, columns, rows);
        let body = frame.join("\n");
        match columns {
            // git's path order (m6-conflicts/25): `gone.txt` first.
            80 => assert!(
                frame[1].contains("✓ gone.txt   ▸ notes.txt 2/2 open"),
                "{body}"
            ),
            200 | 187 => {
                // The list's column only: left of its `│`.
                let at = |name: &str| {
                    frame.iter().position(|row| {
                        row.split('│')
                            .next()
                            .is_some_and(|list| list.contains(name))
                    })
                };
                assert!(
                    frame[1].starts_with("Clashing files")
                        && at("gone.txt").is_some()
                        && at("gone.txt") < at("notes.txt"),
                    "{body}"
                );
            }
            _ => assert_eq!(
                [list_row(&frame, "gone.txt"), list_row(&frame, "notes.txt")],
                ["✓ gone.txt done", "▏? notes.txt 2 open"],
                "{body}"
            ),
        }
        crate::captures::check_shot(
            &format!("m6/fix-file-list-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
    drop(state);
    app.close();
    drop(scratch);
}
