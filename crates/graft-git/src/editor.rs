//! Which editor git would open: `git var GIT_EDITOR`.
//!
//! One spawn instead of four lookups. `git var` applies git's own precedence
//! — `$GIT_EDITOR`, `core.editor`, `$VISUAL`, `$EDITOR`, then the build's
//! fallback — so Graft's `Ctrl+E` and `e` open whatever `git commit` would
//! have opened, including the `[core] editor` a user set years ago and has
//! forgotten about.
//!
//! **Why it lives here and not in `graft`.** The app runs this on the frame
//! thread, because only the thread holding the terminal can hand it over, and
//! `crates/graft/tests/it/seam_rule.rs` forbids a [`GitInvocation`] anywhere
//! in `crates/graft/src` outside `jobs/`. Left as a bare
//! `Command::new("git")` it was the one git spawn in the program that missed
//! the fixed environment, the output cap and the stop policy every other one
//! goes through (m3/05 review, Major 8).

use std::path::Path;

use crate::run::{GitArg, GitInvocation, RunError};

/// Why the editor could not be worked out.
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    /// git could not be run, or said no.
    #[error("{0}")]
    Run(#[from] RunError),
    /// git ran and named nothing. Possible with `GIT_EDITOR=` set empty.
    #[error("git var GIT_EDITOR named no editor")]
    NoEditor,
}

/// The editor command git would run, resolved in `directory`.
///
/// The answer is a **shell command**, not a program name: `emacs -nw`,
/// `code --wait` and `'/Applications/Sublime Text.app/…/subl' -w` are all
/// normal. Splitting it is the caller's problem — see
/// `graft::event_loop::outside::editor_command`, which follows git's rule
/// about when a shell is needed.
///
/// # Errors
/// [`EditorError::Run`] if git is missing, unstartable or exits non-zero;
/// [`EditorError::NoEditor`] if it ran and printed nothing.
pub fn git_editor(directory: &Path) -> Result<String, EditorError> {
    let output = GitInvocation::in_directory(directory)
        .args([GitArg::Flag("var"), GitArg::Flag("GIT_EDITOR")])
        // Never stopped: it is one config read, and the caller is a frame
        // thread that is about to block on the editor anyway.
        .run(&())?;
    let editor = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if editor.is_empty() {
        return Err(EditorError::NoEditor);
    }
    Ok(editor)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// git always has an answer here — the build's fallback if nothing else
    /// — so what this proves is that the invocation reaches git at all and
    /// that the fixed environment `run` sets does not stop it.
    #[test]
    fn git_names_an_editor_for_any_directory_it_will_run_in() {
        let temp = tempfile::TempDir::with_prefix("graft-editor-").expect("a temp dir");

        let editor = git_editor(temp.path()).expect("git answers");

        assert!(!editor.trim().is_empty(), "got {editor:?}");
    }

    /// And a directory git will not run in is an error with git's own words
    /// in it, not a guess.
    #[test]
    fn a_directory_that_is_not_there_is_reported_not_guessed_at() {
        let missing = Path::new("/graft-no-such-directory-ever");

        let error = git_editor(missing).expect_err("git cannot run there");

        assert!(matches!(error, EditorError::Run(_)), "{error}");
    }
}
