//! The words the scenarios are written in: key presses, the setups a
//! fixture takes, and the facts a step or a verdict checks — on the UI
//! state, on a truth, on the journal.

use std::path::{Path, PathBuf};

use graft::update::{AppState, Status};
use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::journal::{EntryStatus, JournalEntry};

use super::fixture::Fixture;
use super::steps::{Step, Wait};
use super::truth::Truth;
use crate::repo_truth::{check, run_git};

pub(super) fn key(code: KeyCode) -> Step {
    Step::Key(KeyPress::plain(code))
}

pub(super) fn ch(c: char) -> KeyCode {
    KeyCode::Char(c)
}

pub(super) fn plain(codes: &[KeyCode]) -> Vec<KeyPress> {
    codes.iter().map(|code| KeyPress::plain(*code)).collect()
}

pub(super) fn ctrl(c: char) -> KeyPress {
    KeyPress {
        code: KeyCode::Char(c),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::default()
        },
    }
}

pub(super) fn chain(parts: Vec<Vec<Step>>) -> Vec<Step> {
    parts.into_iter().flatten().collect()
}

/// `c`, a subject, `[ Commit ]`, the write, then everything re-read.
pub(super) fn commit_by_key(subject: &'static str) -> Vec<Step> {
    vec![
        key(ch('c')),
        Step::Type(subject),
        Step::CommitButton,
        Step::Write("the commit"),
        Step::WaitFor(Wait::Ready),
    ]
}

/// `P`, through the box, to a successful push.
pub(super) fn push_by_key() -> Vec<Step> {
    vec![key(ch('P')), Step::Confirm, Step::Write("the push")]
}

/// Stage a new file before the app opens, so its first status read has it
/// and `c` has something to commit.
pub(super) fn stage_pushed_txt(fixture: &Fixture) -> Result<(), String> {
    std::fs::write(fixture.work().join("pushed.txt"), "pushed\n")
        .map_err(|why| format!("pushed.txt: {why}"))?;
    run_git(fixture.work(), &["add", "pushed.txt"])?;
    Ok(())
}

pub(super) fn teammate_commits(fixture: &Fixture) -> Result<(), String> {
    fixture
        .local
        .commit_on_remote("theirs")
        .map(drop)
        .map_err(|why| format!("the teammate's commit: {why}"))
}

pub(super) fn ours_locally(fixture: &Fixture) -> Result<(), String> {
    fixture
        .local
        .commit_locally("ours")
        .map(drop)
        .map_err(|why| format!("the local commit: {why}"))
}

pub(super) fn amend_locally(fixture: &Fixture) -> Result<(), String> {
    fixture
        .git(
            fixture.work(),
            &["commit", "--amend", "-q", "-m", "amended"],
        )
        .map(drop)
}

pub(super) fn sticky_containing(state: &AppState, texts: &[&str]) -> Result<(), String> {
    let Status::Sticky(line) = &state.status else {
        return Err(format!("not a sticky failure: {:?}", state.status));
    };
    for text in texts {
        check(line.contains(text), || {
            format!("the line lacks `{text}`: {line}")
        })?;
    }
    Ok(())
}

pub(super) fn done_containing(state: &AppState, text: &str) -> Result<(), String> {
    let Status::Done(line) = &state.status else {
        return Err(format!("not a Done line: {:?}", state.status));
    };
    check(line.contains(text), || {
        format!("the line lacks `{text}`: {line}")
    })
}

pub(super) fn box_title_starts(state: &AppState, text: &str) -> Result<(), String> {
    let title = state
        .confirm
        .as_ref()
        .map(|pending| pending.view.title.clone())
        .ok_or_else(|| format!("no box is open; status {:?}", state.status))?;
    check(title.starts_with(text), || {
        format!("the box is titled `{title}`, not `{text}…`")
    })
}

/// Every text is on some line of the open box's message — the lines
/// `TESTING.md` quotes, so a wording change fails the walk, not just the
/// capture.
pub(super) fn box_message_contains(state: &AppState, texts: &[&str]) -> Result<(), String> {
    let view = &state
        .confirm
        .as_ref()
        .ok_or_else(|| format!("no box is open; status {:?}", state.status))?
        .view;
    for text in texts {
        check(view.message.iter().any(|line| line.contains(text)), || {
            format!("the box's message lacks `{text}`: {:?}", view.message)
        })?;
    }
    Ok(())
}

pub(super) fn no_box(state: &AppState) -> Result<(), String> {
    check(state.confirm.is_none(), || {
        format!(
            "a box opened: {:?}",
            state.confirm.as_ref().map(|pending| &pending.view.title)
        )
    })
}

pub(super) fn prompt_is(state: &AppState, masked: bool, naming: &str) -> Result<(), String> {
    let prompt = state
        .credential
        .as_ref()
        .ok_or_else(|| format!("no credential prompt is open; status {:?}", state.status))?;
    check(prompt.masked() == masked, || {
        format!("the prompt's mask is {}, wanted {masked}", prompt.masked())
    })?;
    let text = prompt.prompt.as_str();
    check(text.contains(naming), || {
        format!("the prompt does not name `{naming}`: {text}")
    })
}

pub(super) fn no_prompt(state: &AppState) -> Result<(), String> {
    check(state.credential.is_none(), || {
        "the credential prompt is still open".to_owned()
    })
}

pub(super) fn same_bare(what: &str, a: &Truth, b: &Truth) -> Result<(), String> {
    check(a.bare == b.bare, || {
        let mut lines = Vec::new();
        for (name, was) in &a.bare {
            if b.bare.get(name) != Some(was) {
                lines.push(format!("{name}: {was} → {:?}", b.bare.get(name)));
            }
        }
        for name in b.bare.keys() {
            if !a.bare.contains_key(name) {
                lines.push(format!("{name}: new"));
            }
        }
        format!("{what}: the bare moved: {}", lines.join(" | "))
    })
}

pub(super) fn bare_ref<'a>(truth: &'a Truth, name: &str) -> Result<&'a str, String> {
    truth
        .bare
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("the bare has no {name}"))
}

pub(super) fn work_ref<'a>(truth: &'a Truth, name: &str) -> Result<&'a str, String> {
    truth
        .work
        .refs
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("the work clone has no {name}"))
}

pub(super) fn head(truth: &Truth) -> Result<&str, String> {
    truth
        .work
        .head
        .as_deref()
        .ok_or_else(|| "the work clone has no HEAD".to_owned())
}

pub(super) fn last_entry(truth: &Truth) -> Result<&JournalEntry, String> {
    truth
        .journal
        .last()
        .ok_or_else(|| "the journal is empty".to_owned())
}

pub(super) fn aborted_cancelled(entry: &JournalEntry) -> Result<(), String> {
    check(
        matches!(&entry.status, EntryStatus::Aborted { reason } if reason.contains("cancelled")),
        || format!("the entry is {:?}, not Aborted(cancelled)", entry.status),
    )
}

/// Every `*.lock` under `.git`.
pub(super) fn locks(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(locks(&path));
        } else if path.extension().is_some_and(|ext| ext == "lock") {
            found.push(path);
        }
    }
    found
}
