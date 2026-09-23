//! M6's promise, part one: *a real conflict is resolvable without leaving
//! the app* (m6-conflicts/07). Every scenario here starts from
//! [`Shape::StoppedMerge`] — thirteen unmerged paths, one per shape git
//! has — and resolves by keys in the resolver, then reads the result back
//! from git: bytes against what `git checkout --ours|--theirs` writes on a
//! fresh copy of the same fixture, index entries against the raw git
//! commands of the whole-file table, `ls-files -u`, the merge commit's
//! parents, the journal file.

use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::EntryStatus;
use graft_fixtures::Shape;

use super::fixture::{
    EDITED, Fixture, Remote, TOOL_WROTE, as_built, with_a_mergetool, with_an_editor,
};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::truth::{Side, resolved_bytes, side_bytes, unmerged};
use super::vocabulary::{branch, ch, chain, key, plain, same_files, status_containing, undo};
use crate::repo_truth::{check, run_git};

pub(super) fn resolving() -> Vec<Scenario> {
    vec![
        merge_resolved_with_the_keys(),
        every_whole_file_shape(),
        whole_file_card_glimpses_a_large_file(),
        binary_card_shows_its_size(),
        crlf_and_no_final_newline_survive(),
        zdiff3_and_base_on_demand(),
        n_past_the_last_block_says_no_more(),
        start_over_brings_the_markers_back(),
        editor_edits_are_never_clobbered(),
        mergetool_hand_over_and_back(),
    ]
}

// ───────────────────────────── vocabulary ─────────────────────────────

/// A take (`o`, `t`, `b`) on the cursor block, and its answer.
fn take(side: char) -> Vec<Step> {
    vec![key(ch(side)), Step::WaitFor(Wait::ConflictOpened)]
}

/// `⏎`: the stage, run to its answer, and git no longer listing `path`.
/// `Settle`, not `Write`: the last file's answer is a `Sticky`
/// (`all conflicts resolved · c continue`), not a `Done`.
fn mark_resolved(path: &'static str) -> Vec<Step> {
    vec![
        key(KeyCode::Enter),
        Step::Settle("the stage behind ⏎"),
        Step::WaitFor(Wait::ResolvedPath(path)),
    ]
}

/// A whole-file card's `o` or `t`: one key resolves *and* stages.
fn card(path: &'static str, side: char) -> Vec<Step> {
    vec![
        Step::Resolve(path),
        key(ch(side)),
        Step::Settle("the card's resolution"),
        Step::WaitFor(Wait::ResolvedPath(path)),
    ]
}

/// A text file with one block: open it, take `side`, `⏎`.
fn one_block(path: &'static str, side: char) -> Vec<Step> {
    chain(vec![
        vec![Step::Resolve(path)],
        take(side),
        mark_resolved(path),
    ])
}

/// A fresh copy of the fixture the scenario started from — still stopped,
/// so git can say what each side is.
fn pristine() -> Result<Fixture, String> {
    Fixture::build(Shape::StoppedMerge, Remote::None)
}

/// Which side a block took, for [`blockwise`].
#[derive(Clone, Copy)]
enum Took {
    Ours,
    Theirs,
    Both,
}

/// The bytes a file of one-line blocks has once block *i* took `takes[i]`:
/// `ours` and `theirs` (git's own `checkout --ours|--theirs` bytes) line by
/// line, the plain lines where they agree, the take where they differ.
/// Fails, naming the counts, if the two sides do not line up one line per
/// block — the shape this reading of "per block" depends on.
fn blockwise(ours: &[u8], theirs: &[u8], takes: &[Took]) -> Result<Vec<u8>, String> {
    let ours: Vec<&[u8]> = ours.split_inclusive(|byte| *byte == b'\n').collect();
    let theirs: Vec<&[u8]> = theirs.split_inclusive(|byte| *byte == b'\n').collect();
    check(ours.len() == theirs.len(), || {
        format!("{} lines ours, {} theirs", ours.len(), theirs.len())
    })?;
    let mut takes = takes.iter();
    let mut out = Vec::new();
    for (our, their) in ours.iter().zip(&theirs) {
        if our == their {
            out.extend_from_slice(our);
            continue;
        }
        match takes.next() {
            Some(Took::Ours) => out.extend_from_slice(our),
            Some(Took::Theirs) => out.extend_from_slice(their),
            Some(Took::Both) => {
                out.extend_from_slice(our);
                out.extend_from_slice(their);
            }
            None => return Err("more differing lines than blocks".to_owned()),
        }
    }
    check(takes.next().is_none(), || {
        "fewer differing lines than blocks".to_owned()
    })?;
    Ok(out)
}

/// The resolved bytes of `path` equal git's `side` of it.
fn is_side(fixture: &Fixture, pristine: &Fixture, path: &str, side: Side) -> Result<(), String> {
    let got = resolved_bytes(fixture.repo(), path)?;
    let want = side_bytes(pristine.repo(), path, side)?;
    check(got == want, || {
        format!(
            "{path} is {:?}, git's {side:?} is {:?}",
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(&want)
        )
    })
}

/// How many `<<<<<<<` marker lines `bytes` holds.
fn markers(bytes: &[u8]) -> usize {
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| line.starts_with(b"<<<<<<< "))
        .count()
}

/// The prelude keys that open the resolver on the `nth` row of the
/// changes pane's CONFLICTS list (git's status order, from 0) at launch
/// of a shell-begun [`Shape::StoppedMerge`]: `k` onto the
/// uncommitted-changes row, `⏎` into the pane, `j` down, `⏎`.
fn open_nth(nth: usize) -> Vec<KeyCode> {
    let mut keys = vec![ch('k'), KeyCode::Enter];
    keys.extend(std::iter::repeat_n(ch('j'), nth));
    keys.push(KeyCode::Enter);
    keys
}

/// The thirteen unmerged paths, in git's status order, with how the walk
/// resolves each when the scenario is not about that file: every card by
/// its first key, every text block by `o`. `(path, blocks, key)`; `0`
/// blocks means a whole-file card.
const STOPPED: [(&str, usize, char); 13] = [
    ("added-by-them.txt", 0, 't'),
    ("added-by-us.txt", 0, 'o'),
    ("both-added.txt", 1, 'o'),
    ("both-deleted.txt", 0, 'o'),
    ("both.txt", 1, 'o'),
    ("crlf.txt", 1, 'o'),
    ("md.md", 1, 'o'),
    ("nofinal.txt", 1, 'o'),
    ("ours-deleted.txt", 0, 't'),
    ("pic.bin", 0, 'o'),
    ("theirs-deleted.txt", 0, 'o'),
    ("three.txt", 3, 'o'),
    ("zdiff3.txt", 1, 'o'),
];

/// Where `path` sits in git's status order, for [`open_nth`].
fn nth(path: &str) -> usize {
    STOPPED
        .iter()
        .position(|(name, _, _)| *name == path)
        .unwrap_or_else(|| panic!("{path} is not one of the stopped merge's paths"))
}

/// Resolve every stopped path except `done`, by [`STOPPED`]'s key.
fn resolve_the_rest(done: &[&str]) -> Vec<Step> {
    let mut steps = Vec::new();
    for (path, blocks, side) in STOPPED {
        if done.contains(&path) {
            continue;
        }
        if blocks == 0 {
            steps.extend(card(path, side));
            continue;
        }
        steps.push(Step::Resolve(path));
        for block in 0..blocks {
            steps.extend(take(side));
            if block + 1 < blocks {
                steps.push(key(ch('n')));
            }
        }
        steps.extend(mark_resolved(path));
    }
    steps
}

// ───────────────────────────── the scenarios ─────────────────────────────

/// Two files by the keys — `both.txt` ours; `three.txt` ours, theirs,
/// both — the refusal while blocks are open, everything else resolved,
/// then `c` where the line offers it and `c` from the banner: the ticket
/// wants a merge commit whose parents are main and feature. Red today on
/// findings F4 (`c` on the pane opens the commit editor) and F5 (the
/// fixture's merge was begun in the shell, and `c` refuses it) — the
/// ticket's `u` after the commit is covered by `u-right-after-continue-
/// is-accepted` on a merge Graft began.
#[allow(
    clippy::too_many_lines,
    reason = "one scenario: its steps and its verdict, read top to bottom"
)]
fn merge_resolved_with_the_keys() -> Scenario {
    Scenario {
        name: "merge-resolved-with-the-keys",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (120, 40),
        setup: as_built,
        steps: chain(vec![
            vec![Step::WaitFor(Wait::Ready)],
            one_block("both.txt", 'o'),
            vec![Step::Resolve("three.txt")],
            take('o'),
            vec![
                key(KeyCode::Enter),
                Step::See("⏎ with two blocks open is refused in words", |state| {
                    status_containing(state, "2 conflicts still open in three.txt")?;
                    status_containing(state, "n next")?;
                    check(state.operating.is_none(), || {
                        "⏎ staged a file with blocks still open".to_owned()
                    })
                }),
                key(ch('n')),
            ],
            take('t'),
            vec![key(ch('n'))],
            take('b'),
            vec![Step::Mark("three taken")],
            mark_resolved("three.txt"),
            resolve_the_rest(&["both.txt", "three.txt"]),
            vec![
                Step::See("the last file closes to the banner", |state| {
                    status_containing(state, "all conflicts resolved · c continue")?;
                    // Opened from the pane, the resolver closes back to it
                    // (the rebase scenarios, opened from the banner, close
                    // to the banner). The line offers `c`; the next key is
                    // `c`, pressed where the user now is.
                    check(state.resolver.is_none(), || {
                        format!(
                            "resolver still open on {:?}",
                            state.resolver.as_ref().map(|open| open.path.clone())
                        )
                    })
                }),
                Step::Mark("resolved"),
                // `c` where the line offers it — the pane — then, whatever
                // that did, `⇥` round to the banner and `c` from there.
                key(ch('c')),
                Step::Settle("c on the pane"),
                Step::Mark("c on the pane"),
                // What `c` on the pane does today: the commit editor, which
                // itself says `merge in progress — esc, then c continue`.
                Step::See("c on the pane opened the commit editor", |state| {
                    check(state.commit_editor.is_some(), || {
                        format!("no commit editor; status {:?}", state.status)
                    })
                }),
                key(KeyCode::Escape),
                Step::FocusBanner,
                key(ch('c')),
                Step::Settle("c from the banner"),
                Step::Mark("continued"),
            ],
        ]),
        expect: |fixture, marks| {
            let pristine = pristine()?;
            let three = marks.get("three taken")?;
            let ours = side_bytes(pristine.repo(), "three.txt", Side::Ours)?;
            let theirs = side_bytes(pristine.repo(), "three.txt", Side::Theirs)?;
            let want = blockwise(&ours, &theirs, &[Took::Ours, Took::Theirs, Took::Both])?;
            let got = three
                .repo
                .files
                .get(std::path::Path::new("three.txt"))
                .ok_or("three.txt is gone")?;
            check(*got == want, || {
                format!(
                    "three.txt is {:?}, not {:?}",
                    String::from_utf8_lossy(got),
                    String::from_utf8_lossy(&want)
                )
            })?;
            // Every path resolved: git has no unmerged entry left, and
            // `both.txt` is git's own `--ours`.
            let resolved = marks.get("resolved")?;
            check(resolved.index_tree.is_some(), || {
                "git still lists unmerged entries after the last ⏎".to_owned()
            })?;
            let resolved_both = resolved
                .repo
                .files
                .get(std::path::Path::new("both.txt"))
                .ok_or("both.txt is gone")?;
            check(
                *resolved_both == side_bytes(pristine.repo(), "both.txt", Side::Ours)?,
                || "both.txt is not git's ours".to_owned(),
            )?;
            // The ticket's `c` → a merge commit. Last, so everything above is
            // proven whatever these say. Two findings in the ticket: F4 (the
            // line offers `c continue` on the pane, where `c` opens the commit
            // editor) and F5 (the fixture's merge was begun in the shell, and
            // `c` from the banner refuses to finish it).
            let on_the_pane = marks.get("c on the pane")?;
            check(on_the_pane.in_progress.is_none(), || {
                "F4: `all conflicts resolved · c continue` was on screen, and `c` on the \
                 pane opened the commit editor; the merge is still in progress"
                    .to_owned()
            })?;
            let continued = marks.get("continued")?;
            let parents = run_git(
                fixture.repo(),
                &["rev-list", "--parents", "-n", "1", "main"],
            )?;
            let parents: Vec<&str> = parents.split_whitespace().skip(1).collect();
            let before_main = branch(&marks.before, "main")?;
            let feature = branch(&marks.before, "feature")?;
            check(
                continued.in_progress.is_none()
                    && parents == [before_main.as_str(), feature.as_str()],
                || {
                    format!(
                        "F5: `c` from the banner left git saying {:?}; main's parents {parents:?}; \
                         the app said {:?}",
                        continued.in_progress,
                        continued.statuses.last()
                    )
                },
            )
        },
        capture: Some(Capture {
            stem: "m6-05-still-open-refusal",
            headless: vec![],
            keys: plain(&[open_nth(nth("three.txt")), vec![ch('o'), KeyCode::Enter]].concat()),
            on_screen: "conflicts still open",
            when: When::Settled,
        }),
    }
}

/// The seven whole-file shapes git has — each resolved by the card's `o`
/// or `t` (the `AA` text file by its one block) — leave the index and the
/// tree exactly as the raw git commands of the whole-file table do on a
/// fresh copy.
#[allow(
    clippy::too_many_lines,
    reason = "one scenario: its steps and its verdict, read top to bottom"
)]
fn every_whole_file_shape() -> Scenario {
    Scenario {
        name: "every-whole-file-shape",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (120, 40),
        setup: as_built,
        steps: chain(vec![
            vec![Step::WaitFor(Wait::Ready)],
            card("theirs-deleted.txt", 'o'),
            card("ours-deleted.txt", 'o'),
            card("added-by-us.txt", 't'),
            card("added-by-them.txt", 't'),
            card("both-deleted.txt", 'o'),
            card("pic.bin", 't'),
            one_block("both-added.txt", 'o'),
            vec![Step::Mark("resolved")],
        ]),
        expect: |fixture, _| {
            let raw = pristine()?;
            // G3's table (graft-ops MAP §conflict), run by hand.
            for args in [
                &["add", "--", "theirs-deleted.txt"][..],
                &["rm", "--quiet", "--", "ours-deleted.txt"],
                &["rm", "--quiet", "--", "added-by-us.txt"],
                &["add", "--", "added-by-them.txt"],
                &["rm", "--quiet", "--", "both-deleted.txt"],
                &["checkout", "--theirs", "--", "pic.bin"],
                &["add", "--", "pic.bin"],
                &["checkout", "--ours", "--", "both-added.txt"],
                &["add", "--", "both-added.txt"],
            ] {
                run_git(raw.repo(), args)?;
            }
            let paths = [
                "theirs-deleted.txt",
                "ours-deleted.txt",
                "added-by-us.txt",
                "added-by-them.txt",
                "both-deleted.txt",
                "pic.bin",
                "both-added.txt",
            ];
            let mut listing = vec!["ls-files", "-s", "--"];
            listing.extend(paths);
            let app = run_git(fixture.repo(), &listing)?;
            let git = run_git(raw.repo(), &listing)?;
            check(app == git, || {
                format!("the index after the keys:\n{app}\nafter raw git:\n{git}")
            })?;
            // `None` is "not on disk" — what a delete leaves; any other read
            // failure is an error, not a match.
            let on_disk = |repo: &std::path::Path, path: &str| match std::fs::read(repo.join(path))
            {
                Ok(bytes) => Ok(Some(bytes)),
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(why) => Err(format!("{path}: {why}")),
            };
            for path in paths {
                let app = on_disk(fixture.repo(), path)?;
                let git = on_disk(raw.repo(), path)?;
                check(app == git, || {
                    format!(
                        "{path}: on disk after the keys {:?} bytes, after raw git {:?}",
                        app.as_ref().map(Vec::len),
                        git.as_ref().map(Vec::len)
                    )
                })?;
            }
            Ok(())
        },
        capture: Some(Capture {
            stem: "m6-02-whole-file-card",
            headless: vec![],
            keys: plain(&open_nth(nth("theirs-deleted.txt"))),
            on_screen: "deleted by them",
            when: When::Settled,
        }),
    }
}

/// How many lines [`a_large_kept_file`] writes: over the card's 500.
const LARGE: usize = 600;

/// [`Shape::StoppedMerge`] with the kept side of `theirs-deleted.txt` (a
/// `UD`: our change, their delete) grown to [`LARGE`] lines in the working
/// tree before the app opens — a card over a file too long to show whole.
fn a_large_kept_file(fixture: &Fixture) -> Result<(), String> {
    let body = (1..=LARGE)
        .map(|n| format!("kept line {n}\n"))
        .collect::<Vec<_>>()
        .concat();
    fixture.dirty("theirs-deleted.txt", &body)
}

/// The card over a large file shows its head and says so in the note row;
/// `t` deletes it as the table says.
fn whole_file_card_glimpses_a_large_file() -> Scenario {
    Scenario {
        name: "whole-file-card-glimpses-a-large-file",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (120, 40),
        setup: a_large_kept_file,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::Resolve("theirs-deleted.txt"),
            Step::See("the card holds the file's head, cut", |state| {
                let card = state
                    .resolver
                    .as_ref()
                    .and_then(|open| open.whole_file.as_ref())
                    .ok_or("no whole-file card")?;
                let seen = card.on_disk.as_ref().ok_or("the card read nothing")?;
                check(seen.cut && seen.head.len() == 500, || {
                    format!("{} lines read, cut {}", seen.head.len(), seen.cut)
                })
            }),
            // The note is the card's last row, under all 500 lines: `G`.
            key(ch('G')),
            Step::See("G shows the unnumbered note row", |state| {
                let rows = crate::captures::frame(state, 120, 40);
                check(
                    rows.iter().any(|row| row.contains("first 500 lines of")),
                    || format!("no note row:\n{}", rows.join("\n")),
                )
            }),
            key(ch('t')),
            Step::Settle("the card's resolution"),
            Step::WaitFor(Wait::ResolvedPath("theirs-deleted.txt")),
            Step::Mark("deleted"),
        ],
        expect: |fixture, _| {
            check(!fixture.repo().join("theirs-deleted.txt").exists(), || {
                "t kept the file".to_owned()
            })?;
            let entry = run_git(
                fixture.repo(),
                &["ls-files", "-s", "--", "theirs-deleted.txt"],
            )?;
            check(entry.trim().is_empty(), || {
                format!("the index still has {entry}")
            })
        },
        capture: Some(Capture {
            stem: "m6-03-card-large-file",
            headless: vec![],
            keys: plain(&[open_nth(nth("theirs-deleted.txt")), vec![ch('G')]].concat()),
            on_screen: "first 500 lines of",
            when: When::Settled,
        }),
    }
}

/// A binary `UU` gets a card, not a document: its size in place of its
/// bytes; `o` leaves git's ours.
fn binary_card_shows_its_size() -> Scenario {
    Scenario {
        name: "binary-card-shows-its-size",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (120, 40),
        setup: as_built,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::Resolve("pic.bin"),
            Step::See("a binary card with its size", |state| {
                let card = state
                    .resolver
                    .as_ref()
                    .and_then(|open| open.whole_file.as_ref())
                    .ok_or("no whole-file card")?;
                check(card.binary, || "pic.bin is not called binary".to_owned())?;
                let rows = crate::captures::frame(state, 120, 40);
                check(rows.iter().any(|row| row.contains("binary ·")), || {
                    format!("no `binary ·` note:\n{}", rows.join("\n"))
                })
            }),
            key(ch('o')),
            Step::Settle("the card's resolution"),
            Step::WaitFor(Wait::ResolvedPath("pic.bin")),
        ],
        expect: |fixture, _| is_side(fixture, &pristine()?, "pic.bin", Side::Ours),
        capture: Some(Capture {
            stem: "m6-04-card-binary",
            headless: vec![],
            keys: plain(&open_nth(nth("pic.bin"))),
            on_screen: "binary ·",
            when: When::Settled,
        }),
    }
}

/// CRLF throughout, and a file with no final newline: the takes keep
/// every byte git's own `checkout` would.
fn crlf_and_no_final_newline_survive() -> Scenario {
    Scenario {
        name: "crlf-and-no-final-newline-survive",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (80, 24),
        setup: as_built,
        steps: chain(vec![
            vec![Step::WaitFor(Wait::Ready)],
            one_block("crlf.txt", 't'),
            one_block("nofinal.txt", 'o'),
        ]),
        expect: |fixture, _| {
            let pristine = pristine()?;
            is_side(fixture, &pristine, "crlf.txt", Side::Theirs)?;
            is_side(fixture, &pristine, "nofinal.txt", Side::Ours)?;
            let crlf = resolved_bytes(fixture.repo(), "crlf.txt")?;
            check(crlf.windows(2).any(|pair| pair == b"\r\n"), || {
                "crlf.txt lost its CRLF".to_owned()
            })?;
            let nofinal = resolved_bytes(fixture.repo(), "nofinal.txt")?;
            check(nofinal.last() != Some(&b'\n'), || {
                "nofinal.txt gained a final newline".to_owned()
            })
        },
        capture: None,
    }
}

/// The open block's base, from wherever git keeps it: `zdiff3.txt`'s own
/// markers carry it (no `B`); `B` fetches all three of `three.txt`'s; an
/// `AA` has none, and says so. Nothing is written.
fn zdiff3_and_base_on_demand() -> Scenario {
    Scenario {
        name: "zdiff3-and-base-on-demand",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (120, 40),
        setup: as_built,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::Resolve("zdiff3.txt"),
            Step::See("zdiff3's block carries its base, B unpressed", |state| {
                let open = state.resolver.as_ref().ok_or("no resolver")?;
                check(!open.base_shown, || "B is on already".to_owned())?;
                check(bases(state) == (1, 1), || {
                    format!("(open blocks, with a base) = {:?}", bases(state))
                })
            }),
            Step::Resolve("three.txt"),
            Step::See("three.txt's blocks carry no base yet", |state| {
                check(bases(state) == (3, 0), || {
                    format!("(open blocks, with a base) = {:?}", bases(state))
                })
            }),
            key(ch('B')),
            Step::WaitFor(Wait::ConflictOpened),
            Step::See("B brought all three bases", |state| {
                let open = state.resolver.as_ref().ok_or("no resolver")?;
                check(open.base_shown, || "B did not turn base on".to_owned())?;
                check(bases(state) == (3, 3), || {
                    format!("(open blocks, with a base) = {:?}", bases(state))
                })
            }),
            Step::Resolve("both-added.txt"),
            key(ch('B')),
            Step::WaitFor(Wait::ConflictOpened),
            Step::See("an AA says it has no base", |state| {
                let open = state.resolver.as_ref().ok_or("no resolver")?;
                check(
                    open.notice.as_ref().is_some_and(|(words, problem)| {
                        words == "no base (added on both sides)" && !problem
                    }),
                    || format!("the notice is {:?}", open.notice),
                )
            }),
            Step::Mark("looked"),
        ],
        expect: |_, marks| {
            let looked = marks.get("looked")?;
            same_files("looking at bases", &marks.before, looked)?;
            check(looked.repo.index == marks.before.repo.index, || {
                "looking at bases changed the index".to_owned()
            })
        },
        capture: Some(Capture {
            stem: "m6-01-resolver-three",
            headless: vec![],
            keys: plain(&[open_nth(nth("three.txt")), vec![ch('n'), ch('t'), ch('B')]].concat()),
            on_screen: "took theirs",
            when: When::Settled,
        }),
    }
}

/// `(open blocks, open blocks with a base)` in the document on screen: the
/// file's own `zdiff3` base (`block.base`) or the one `B` reproduced from
/// git's stages (`block.reproduced`, `graft_git::conflict::attach_base`).
fn bases(state: &graft::update::AppState) -> (usize, usize) {
    use graft_git::conflict::Region;
    let Some(file) = state.resolver.as_ref().and_then(|open| open.file.as_ref()) else {
        return (0, 0);
    };
    file.regions
        .iter()
        .fold((0, 0), |(open, based), region| match region {
            Region::Conflict(block) => (
                open + 1,
                based + usize::from(block.base.is_some() || block.reproduced.is_some()),
            ),
            _ => (open, based),
        })
}

/// On the last conflicted file in git's order, `N` before the first open
/// block and `n` past the last have nowhere to go, and say what `⏎` does
/// instead. Nothing is written.
fn n_past_the_last_block_says_no_more() -> Scenario {
    Scenario {
        name: "n-past-the-last-block-says-no-more",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (120, 40),
        setup: as_built,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::Resolve("zdiff3.txt"),
            key(ch('N')),
            Step::See("N before the first block", |state| {
                status_containing(state, "no more conflicts · ⏎ marks zdiff3.txt resolved")
            }),
            key(ch('n')),
            Step::WaitFor(Wait::ConflictOpened),
            Step::See("n past the last block of the last file", |state| {
                status_containing(state, "no more conflicts · ⏎ marks zdiff3.txt resolved")?;
                let on = state.resolver.as_ref().map(|open| open.path.clone());
                check(
                    on.as_deref() == Some(std::path::Path::new("zdiff3.txt")),
                    || format!("the resolver moved to {on:?}"),
                )
            }),
            Step::Mark("walked"),
        ],
        expect: |_, marks| same_files("n and N", &marks.before, marks.get("walked")?),
        capture: Some(Capture {
            stem: "m6-08-no-more-conflicts",
            headless: vec![],
            keys: plain(&[open_nth(nth("zdiff3.txt")), vec![ch('N'), ch('n')]].concat()),
            on_screen: "no more conflicts",
            when: When::Settled,
        }),
    }
}

/// `d` → `Start over` after a take: git's stages for `both.txt` are the
/// ones the stop left (same blob ids), one marker block is on disk, the
/// block is open again.
fn started_over() -> Vec<Step> {
    vec![
        key(ch('d')),
        Step::See("the box asks, naming the file", |state| {
            let title = &state
                .confirm
                .as_ref()
                .ok_or("no start-over box")?
                .view
                .title;
            check(title.contains("Start both.txt over"), || {
                format!("the box is titled {title:?}")
            })
        }),
        Step::Confirm,
        Step::Settle("start over"),
        Step::WaitFor(Wait::ConflictOpened),
        Step::See("the block is open again", |state| {
            let open = state
                .resolver
                .as_ref()
                .and_then(|open| open.file.as_ref())
                .map(|file| file.open());
            check(open == Some(1), || format!("{open:?} blocks open"))
        }),
        // Truth from git, mid-walk: the stages against a fresh build of the
        // same stop (blob ids are content addresses, so the stop's own ids).
        Step::Shell(|fixture| {
            let stopped = unmerged(pristine()?.repo(), "both.txt")?;
            check(stopped.len() == 3, || format!("the stop: {stopped:?}"))?;
            let now = unmerged(fixture.repo(), "both.txt")?;
            check(now == stopped, || {
                format!("stages at the stop {stopped:?}, after d {now:?}")
            })?;
            check(
                markers(&resolved_bytes(fixture.repo(), "both.txt")?) == 1,
                || "both.txt has no markers after d".to_owned(),
            )
        }),
    ]
}

/// A take then `d`: the markers come back with git's own stages. Then the
/// ticket's harder half — a take, `⏎` (staged), back into the same file,
/// `d` — which needs the stage undone first, since a staged path has no
/// `CONFLICTS` row to open it from.
fn start_over_brings_the_markers_back() -> Scenario {
    Scenario {
        name: "start-over-brings-the-markers-back",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (80, 24),
        setup: as_built,
        steps: chain(vec![
            vec![Step::WaitFor(Wait::Ready), Step::Resolve("both.txt")],
            take('o'),
            started_over(),
            vec![Step::Mark("started over")],
            take('o'),
            mark_resolved("both.txt"),
            // `⏎` moved the resolver on to the next file, and it is modal:
            // `Esc` first, then `u` for the stage.
            vec![key(KeyCode::Escape)],
            undo(),
            vec![
                Step::WaitFor(Wait::ConflictedPath("both.txt")),
                Step::Resolve("both.txt"),
            ],
            started_over(),
        ]),
        expect: |fixture, marks| {
            let stages = |truth: &super::truth::Truth| -> Vec<String> {
                truth
                    .repo
                    .index
                    .iter()
                    .filter(|line| line.ends_with("\tboth.txt"))
                    .cloned()
                    .collect()
            };
            let before = stages(&marks.before);
            check(before.len() == 3, || format!("before: {before:?}"))?;
            check(stages(marks.get("started over")?) == before, || {
                "after the first d the stages are not the stop's".to_owned()
            })?;
            let listed = unmerged(fixture.repo(), "both.txt")?;
            check(listed == before, || {
                format!("stages before the take {before:?}, after d {listed:?}")
            })
        },
        capture: Some(Capture {
            stem: "m6-09-start-over-box",
            headless: vec![],
            keys: plain(&[open_nth(nth("both.txt")), vec![ch('d')]].concat()),
            on_screen: "Start both.txt over",
            when: When::Settled,
        }),
    }
}

/// `e` hands `three.txt` to `$EDITOR`, which rewrites its first block by
/// hand; back in the resolver the file is read again, a take lands on what
/// the edit left, and an edit made behind the resolver's back makes the
/// next take refuse and re-read rather than write over it.
#[allow(
    clippy::too_many_lines,
    reason = "one scenario: its steps and its verdict, read top to bottom"
)]
fn editor_edits_are_never_clobbered() -> Scenario {
    Scenario {
        name: "editor-edits-are-never-clobbered",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (80, 24),
        setup: with_an_editor,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::Resolve("three.txt"),
            Step::HandOver("e hands three.txt to $EDITOR", KeyPress::plain(ch('e'))),
            Step::WaitFor(Wait::ConflictOpened),
            Step::See("the re-read shows two open blocks", |state| {
                let open = state
                    .resolver
                    .as_ref()
                    .and_then(|open| open.file.as_ref())
                    .map(|file| file.open());
                check(open == Some(2), || format!("{open:?} blocks open"))
            }),
            Step::Mark("edited"),
            key(ch('o')),
            Step::WaitFor(Wait::ConflictOpened),
            Step::Mark("taken"),
            Step::Shell(|fixture| {
                let bytes = fixture.bytes("three.txt")?;
                let text = String::from_utf8_lossy(&bytes).replace(EDITED, "edited twice");
                fixture.dirty("three.txt", &text)?;
                fixture.dirty(".git/walk-expected-three", &text)
            }),
            key(ch('o')),
            Step::WaitFor(Wait::ConflictOpened),
            // Either answer the ticket allows: the take refuses with
            // `changed on disk` and re-reads, or it lands on the re-read
            // file. What may not happen is a write over the edit — the
            // expectation below reads that from the file.
            Step::Mark("refused"),
        ],
        expect: |fixture, marks| {
            let file = |mark: &str| -> Result<Vec<u8>, String> {
                marks
                    .get(mark)?
                    .repo
                    .files
                    .get(std::path::Path::new("three.txt"))
                    .cloned()
                    .ok_or_else(|| format!("{mark}: three.txt is gone"))
            };
            let edited = file("edited")?;
            let line = format!("{EDITED}\n");
            check(
                edited.windows(line.len()).any(|w| w == line.as_bytes()) && markers(&edited) == 2,
                || format!("after $EDITOR: {:?}", String::from_utf8_lossy(&edited)),
            )?;
            let taken = file("taken")?;
            check(
                taken.windows(line.len()).any(|w| w == line.as_bytes()) && markers(&taken) == 1,
                || format!("after the take: {:?}", String::from_utf8_lossy(&taken)),
            )?;
            let expected = fixture.bytes(".git/walk-expected-three")?;
            let refused = file("refused")?;
            let twice = b"edited twice\n";
            check(
                refused == expected
                    || (refused.windows(twice.len()).any(|w| w == twice) && markers(&refused) == 0),
                || {
                    format!(
                        "the stale take wrote over the edit: {:?}",
                        String::from_utf8_lossy(&refused)
                    )
                },
            )
        },
        capture: None,
    }
}

/// `M` with `merge.tool` set: a tool that resolves and exits 0 closes the
/// resolver with `resolved outside`; one that exits 1 leaves the conflict
/// and says so, and the resolver reads the file again.
#[allow(
    clippy::too_many_lines,
    reason = "one scenario: its steps and its verdict, read top to bottom"
)]
fn mergetool_hand_over_and_back() -> Scenario {
    Scenario {
        name: "mergetool-hand-over-and-back",
        shape: Shape::StoppedMerge,
        remote: Remote::None,
        size: (80, 24),
        setup: with_a_mergetool,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            Step::Resolve("both.txt"),
            Step::HandOver("M hands both.txt to the tool", KeyPress::plain(ch('M'))),
            Step::WaitFor(Wait::ResolvedPath("both.txt")),
            Step::WaitFor(Wait::ResolverClosed),
            Step::See("resolved outside, and closed", |state| {
                status_containing(state, "both.txt resolved outside")?;
                check(state.resolver.is_none(), || {
                    "the resolver is open".to_owned()
                })
            }),
            Step::Resolve("crlf.txt"),
            Step::HandOver("M hands crlf.txt to the tool", KeyPress::plain(ch('M'))),
            Step::WaitFor(Wait::ConflictOpened),
            Step::See(
                "the tool's failure, in words, and the file re-read",
                |state| {
                    status_containing(state, "mergetool exited 1 for crlf.txt")?;
                    let on = state.resolver.as_ref().map(|open| open.path.clone());
                    check(
                        on.as_deref() == Some(std::path::Path::new("crlf.txt")),
                        || format!("the resolver is on {on:?}"),
                    )
                },
            ),
            Step::Mark("failed"),
        ],
        expect: |fixture, marks| {
            check(
                resolved_bytes(fixture.repo(), "both.txt")? == TOOL_WROTE,
                || "both.txt is not what the tool wrote".to_owned(),
            )?;
            check(unmerged(fixture.repo(), "both.txt")?.is_empty(), || {
                "git still lists both.txt".to_owned()
            })?;
            check(unmerged(fixture.repo(), "crlf.txt")?.len() == 3, || {
                "crlf.txt is no longer a three-stage conflict".to_owned()
            })?;
            let failed = marks.get("failed")?;
            check(
                failed.repo.files.get(std::path::Path::new("crlf.txt"))
                    == marks
                        .before
                        .repo
                        .files
                        .get(std::path::Path::new("crlf.txt")),
                || "the failed tool changed crlf.txt".to_owned(),
            )?;
            check(
                failed
                    .journal
                    .iter()
                    .all(|entry| !matches!(entry.status, EntryStatus::Pending)),
                || "a hand-over left a Pending entry".to_owned(),
            )
        },
        capture: None,
    }
}
