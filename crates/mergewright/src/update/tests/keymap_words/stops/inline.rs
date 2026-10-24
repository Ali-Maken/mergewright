//! Stop words: the inline stop words (m8/10k). Moved out of `stops.rs` (m8/21b-5, move-only).

use super::*;

/// One inline stop word (m8/10k): the words a real event draws under
/// `keys`, and those words with each key spelled by `spell`.
struct Inline {
    name: &'static str,
    keys: &'static [&'static str],
    drawn: fn(&Arc<Keymap>) -> String,
    expected: fn(&dyn Fn(&'static str) -> String) -> String,
}

/// `key` pressed at `ctx` as the edge hands it on once it has translated
/// his chord: `dispatch_key` takes the default chord, whatever his keymap.
fn press_at(state: &mut AppState, ctx: ContextId, key: char) {
    assert_eq!(crate::update::edge::context_of(state), ctx);
    let mut commands = Vec::new();
    crate::update::dispatch_key(
        state,
        &mut commands,
        KeyPress::plain(mergewright_core::event::KeyCode::Char(key)),
    );
}

fn job_ids() -> impl FnMut() -> JobId {
    let mut next = 7_000;
    move || {
        next += 1;
        JobId::new(next)
    }
}

/// Launches `op` on `state` and answers it with `answer`, through
/// `update` as the job's reply arrives.
fn answered(
    mut state: AppState,
    op: Operation,
    answer: crate::jobs::operate::Operated,
) -> AppState {
    let mut commands = Vec::new();
    crate::update::operate::launch(&mut state, &mut commands, op, &mut job_ids())
        .expect("nothing else is running");
    let id = state.operating.expect("the launch took the slot");
    crate::update::update(
        &state,
        AppEvent::Produced(id, Product::Operated(Box::new(answer))),
    )
    .0
}

fn done_keys(status: &Status) -> String {
    match status {
        Status::Done {
            keys: Some(keys), ..
        } => keys.clone(),
        other => panic!("a done line with keys: {other:?}"),
    }
}

fn sticky_reason(status: &Status) -> String {
    match status {
        Status::Sticky { reason, .. } => reason.clone(),
        other => panic!("a sticky line: {other:?}"),
    }
}

/// A diff pane on `one_pair_diff`'s hunk, his keymap `keys`.
fn on_a_hunk(keys: &Arc<Keymap>, mode: mergewright_ui::detail::DiffMode) -> AppState {
    let mut state = crate::update::operate::tests::state_on_diff(
        mergewright_ui::detail::WorkingTreeSection::Unstaged,
        crate::update::operate::tests::one_pair_diff(),
        1,
    );
    state.detail.mode = mode;
    state.keymap = Arc::clone(keys);
    state
}

const INLINE: &[Inline] = &[
    Inline {
        name: "outcome: amend at an edit stop (rewrite_stop_tests)",
        keys: &["operation.continue"],
        drawn: |keys| {
            use crate::update::banner::rewrite_stop_tests::{a_amends, amended, edit_stop};
            // `a` pressed as shipped; the words are set when the amend
            // answers, under his keymap.
            let (mut launched, id) = a_amends(&edit_stop(), "wip: tests");
            launched.keymap = Arc::clone(keys);
            done_keys(&amended(&launched, id).status)
        },
        expected: |k| format!("{} continues", k("operation.continue")),
    },
    Inline {
        name: "outcome: a one-commit cherry-pick",
        keys: &["app.undo"],
        drawn: |keys| {
            let state = AppState {
                keymap: Arc::clone(keys),
                ..AppState::default()
            };
            let oid = mergewright_core::oid::Oid::from_bytes(&[7; 20]).expect("20 bytes");
            let op = Operation::CherryPick(mergewright_ops::operation::PickParams {
                commits: vec![oid],
                subject: "fix the bug".to_owned(),
                head: mergewright_ops::operation::Tip {
                    name: "refs/heads/main".to_owned(),
                    oid,
                },
            });
            let done =
                crate::jobs::operate::Operated::Done(crate::update::banner::tests::done_applied());
            done_keys(&answered(state, op, done).status)
        },
        expected: |k| format!("{} undoes", k("app.undo")),
    },
    Inline {
        name: "failure: MarkersRemain on Space at a CONFLICTS row",
        keys: &["changes.edit", "changes.mergetool"],
        drawn: |keys| {
            let state = AppState {
                keymap: Arc::clone(keys),
                ..AppState::default()
            };
            let refused = crate::jobs::operate::Operated::Failed {
                error: "src/auth.rs still has conflict markers".to_owned(),
                refresh: None,
                remote: None,
                refused: Some(Refusal::MarkersRemain {
                    path: PathBuf::from("src/auth.rs"),
                }),
                edits: None,
            };
            keys_of(answered(state, stage(), refused).status)
        },
        expected: |k| {
            format!(
                "{} edit  {} mergetool",
                k("changes.edit"),
                k("changes.mergetool")
            )
        },
    },
    Inline {
        name: "abort box: a merge begun in the shell (banner/tests merge_read)",
        keys: &["app.undo"],
        drawn: |keys| {
            let mut state = crate::update::banner::tests::merge_read(Vec::new(), Some("main"));
            state.keymap = Arc::clone(keys);
            press_at(&mut state, ContextId::Banner, 'a');
            let pending = state
                .confirm
                .as_ref()
                .unwrap_or_else(|| panic!("the abort box: {:?}", state.status));
            pending
                .view
                .message
                .iter()
                .map(|line| line.as_str().to_owned())
                .collect::<Vec<_>>()
                .join(" | ")
        },
        expected: |k| {
            format!(
                "main goes back to before the merge | {} puts the merge back",
                k("app.undo")
            )
        },
    },
    Inline {
        name: "diff_cursor: Space in the side-by-side view",
        keys: &["diff.side_by_side"],
        drawn: |keys| {
            let mut state = on_a_hunk(keys, mergewright_ui::detail::DiffMode::SideBySide);
            let mut commands = Vec::new();
            assert!(crate::update::operate::stage_at_diff_cursor(
                &mut state,
                &mut commands,
                &mut job_ids()
            ));
            sticky_reason(&state.status)
        },
        expected: |k| {
            format!(
                "side by side has no line cursor \u{2014} press {} for the unified view to stage a hunk",
                k("diff.side_by_side")
            )
        },
    },
    Inline {
        name: "diff_cursor: the discard box's backup line",
        keys: &["app.undo"],
        drawn: |keys| {
            let mut state = on_a_hunk(keys, mergewright_ui::detail::DiffMode::Unified);
            assert!(crate::update::operate::request_discard_at_diff_cursor(
                &mut state
            ));
            let pending = state.confirm.as_ref().expect("the discard box");
            pending
                .view
                .message
                .last()
                .map(|line| line.as_str().to_owned())
                .expect("a backup line")
        },
        expected: |k| {
            format!(
                "Mergewright keeps a backup first, so `{}` can undo it afterwards.",
                k("app.undo")
            )
        },
    },
];

/// The six inline stop words (10j ruling 2, m8/10k; the review's empty-subject
/// one left with its field, ease-pass/12), each driven through
/// the event that draws it: under the shipped keymap byte-identical to the
/// pre-10f literal; under the swept keymap each key its new label, as a
/// whole token, and no default letter of a moved action left.
#[test]
fn each_inline_stop_word_spells_the_remapped_key() {
    assert_eq!(INLINE.len(), 6, "the six inline stop words");
    let shipped = Arc::new(Keymap::shipped().clone());
    let swept = swept();
    for site in INLINE {
        assert_eq!(
            (site.drawn)(&shipped),
            (site.expected)(&|id| letter(id).to_owned()),
            "{}: shipped keymap, byte-identical",
            site.name
        );
        let line = (site.drawn)(&swept);
        assert_eq!(
            line,
            (site.expected)(&|id| spelled(&swept, id)),
            "{}: swept keymap",
            site.name
        );
        let bare = (site.expected)(&|_| String::new());
        for id in site.keys {
            let moved = spelled(&swept, id);
            assert_ne!(moved, letter(id), "{}: {id} moved", site.name);
            // The backup line quotes its key in backticks: trimmed too,
            // unless the label itself ends in one.
            let whole = line
                .split_whitespace()
                .filter(|word| {
                    *word == moved
                        || word.trim_matches(|c: char| "(),.;:`".contains(c)) == moved
                        || word.strip_prefix('`').and_then(|w| w.strip_suffix('`'))
                            == Some(moved.as_str())
                })
                .count();
            assert!(
                whole >= 1,
                "{}: {moved} as a whole token: {line}",
                site.name
            );
            assert_eq!(
                whole_tokens(&line, letter(id)),
                whole_tokens(&bare, letter(id)),
                "{}: no default {} left: {line}",
                site.name,
                letter(id)
            );
        }
    }
}

/// m8-polish-and-ship/20 item 17: the `InTheWay` rung is picked by the
/// width of what is drawn — the sanitised path — so a path full of control
/// characters (each drawn as a one-cell placeholder) still fits the row.
#[test]
fn an_in_the_way_rung_is_measured_as_drawn() {
    use unicode_width::UnicodeWidthStr;
    let keys = Keymap::shipped().clone();
    let path = format!("src/{}.rs", "\u{200b}".repeat(30));
    let refused = Refusal::InTheWay {
        doing: "switching to other".to_owned(),
        ignored: Vec::new(),
        untracked: vec![PathBuf::from(path)],
    };
    let Status::Sticky {
        reason, keys: next, ..
    } = crate::update::operate::refusal_status(&stage(), &refused, 80, &keys)
    else {
        panic!("a sticky line");
    };
    let drawn = mergewright_term::sanitize_line(&reason);
    let suffix = next
        .map(|next| format!(" \u{b7} {next}"))
        .unwrap_or_default();
    assert!(
        drawn.as_str().width() + suffix.width() < 80,
        "{drawn}{suffix}"
    );
}
