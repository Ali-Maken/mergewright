//! A secrets-looking file's values, hidden on the commit screen (08s Q1;
//! the commit mock `:116`, `PAYMENT_KEY=••••••••`).
//!
//! The diff text is sanitised in the job (`jobs::detail::assemble`), so the
//! mask goes on as the diff arrives (`update::commit_editor::diff_arrived`):
//! the state never keeps the value past that point, and graft-ui, a capture
//! or a log never sees it.

use graft_term::sanitize_line;
use graft_ui::detail::{DiffView, HunkView, LineKindView, LineView};

/// The mask a secrets file's value is drawn as: eight bullets, whatever the
/// real length, so the mask says nothing about the value either.
pub(crate) const MASK: &str = "••••••••";

/// One diff line of a secrets-looking file: the key kept, the value after
/// the first `=` or `:` → [`MASK`]; no separator → [`MASK`]. Runs before
/// sanitising.
#[must_use]
pub(crate) fn mask_secret_line(line: &str) -> String {
    match line.find(['=', ':']) {
        // The separator is one byte, so `at + 1` is a char boundary.
        Some(at) => format!("{}{MASK}", &line[..=at]),
        None => MASK.to_owned(),
    }
}

/// `view` with every added, removed and context line masked, and each
/// hunk's heading text too when git printed one: git's default funcname
/// takes it from a line of the file, so in a `.env` it is a `KEY=value`
/// line. The `@@` numbers and the line counts stay. Word and syntax spans
/// index the old text, so they go with it.
#[must_use]
pub(crate) fn mask_diff(view: &DiffView) -> DiffView {
    DiffView {
        hunks: view.hunks.iter().map(mask_hunk).collect(),
        ..view.clone()
    }
}

fn mask_hunk(hunk: &HunkView) -> HunkView {
    HunkView {
        heading: mask_heading(hunk.heading.as_str()),
        lines: hunk.lines.iter().map(mask_line).collect(),
    }
}

/// A heading's text, masked; an empty one (git printed none) stays empty.
fn mask_heading(text: &str) -> graft_term::SanitizedText {
    if text.is_empty() {
        sanitize_line(text)
    } else {
        sanitize_line(&mask_secret_line(text))
    }
}

fn mask_line(line: &LineView) -> LineView {
    let text = match line.kind {
        LineKindView::HunkHeading => mask_heading(line.text.as_str()),
        LineKindView::Context | LineKindView::Added | LineKindView::Removed => {
            sanitize_line(&mask_secret_line(line.text.as_str()))
        }
    };
    LineView {
        text,
        words: Vec::new(),
        syntax: Vec::new(),
        ..line.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{MASK, mask_secret_line};

    /// The ticket's table (08s Q1 lines).
    #[test]
    fn a_secret_line_keeps_its_key_and_hides_its_value() {
        let table = [
            ("PAYMENT_KEY=abc123", "PAYMENT_KEY=••••••••"),
            ("token: abc", "token:••••••••"),
            ("a=b=c", "a=••••••••"),
            ("abc123", "••••••••"),
            ("", "••••••••"),
        ];
        for (line, masked) in table {
            assert_eq!(mask_secret_line(line), masked, "{line:?}");
        }
        let long = format!("KEY={}", "v".repeat(200));
        assert_eq!(mask_secret_line(&long), "KEY=••••••••");
        assert_eq!(MASK.chars().count(), 8);
    }
}
