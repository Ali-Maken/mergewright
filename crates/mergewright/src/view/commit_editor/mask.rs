//! A secrets-looking file's values, hidden on the commit screen (08s Q1;
//! the commit mock `:116`, `PAYMENT_KEY=••••••••`).
//!
//! The diff text is sanitised in the job (`jobs::detail::assemble`), so the
//! mask goes on as the diff arrives (`update::commit_editor::diff_arrived`):
//! the state never keeps the value past that point, and mergewright-ui, a capture
//! or a log never sees it.

use mergewright_term::sanitize_line;
use mergewright_ui::detail::{DiffView, HunkView, LineKindView, LineView};

/// The mask a secrets file's value is drawn as: eight bullets, whatever the
/// real length, so the mask says nothing about the value either.
pub(crate) const MASK: &str = "••••••••";

/// One diff line of a secrets-looking file. `<prefix><sep>` is kept and the
/// rest drawn [`MASK`] only when all three hold; otherwise the whole line is
/// [`MASK`] (08v):
/// 1. `sep` is the line's first `=` or `:`;
/// 2. the prefix is a key: optional leading whitespace, optional `export `,
///    a name `[A-Za-z_][A-Za-z0-9_.-]*` of at most 64 chars, optional
///    whitespace;
/// 3. a character follows `sep` and it is not `=`.
///
/// Base64 has no `:` and `=` only at its end (`…AB==`), so every line of a
/// PEM or OpenSSH key masks whole by (2) or (3). Runs before sanitising.
#[must_use]
pub(crate) fn mask_secret_line(line: &str) -> String {
    let Some(at) = line.find(['=', ':']) else {
        return MASK.to_owned();
    };
    // The separator is one byte, so `at + 1` is a char boundary.
    let after = line[at + 1..].chars().next();
    if is_key(&line[..at]) && after.is_some_and(|c| c != '=') {
        format!("{}{MASK}", &line[..=at])
    } else {
        MASK.to_owned()
    }
}

/// Condition 2 of [`mask_secret_line`]: `prefix` is a key name.
fn is_key(prefix: &str) -> bool {
    let trimmed = prefix.trim_start();
    let name = trimmed
        .strip_prefix("export ")
        .unwrap_or(trimmed)
        .trim_end();
    let mut chars = name.chars();
    let starts = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    starts
        && name.len() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
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
fn mask_heading(text: &str) -> mergewright_term::SanitizedText {
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
            // 08v: keys still kept.
            ("export API_KEY=abc", "export API_KEY=••••••••"),
            ("  password: hunter2", "  password:••••••••"),
            ("Proc-Type: 4,ENCRYPTED", "Proc-Type:••••••••"),
        ];
        for (line, masked) in table {
            assert_eq!(mask_secret_line(line), masked, "{line:?}");
        }
        let long = format!("KEY={}", "v".repeat(200));
        assert_eq!(mask_secret_line(&long), "KEY=••••••••");
        assert_eq!(MASK.chars().count(), 8);
    }

    /// 08v: a PEM / OpenSSH key's lines, and anything whose prefix is not a
    /// key, mask whole.
    #[test]
    fn a_key_files_lines_are_masked_whole() {
        let table = [
            "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7",
            "x5dhkAbCdEfGhIDAQAB==",
            "x5dhkAbCdEfGhIDAQAB=",
            "-----BEGIN RSA PRIVATE KEY-----",
            "-----BEGIN OPENSSH PRIVATE KEY-----",
            "b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW",
            "KEY=",
            "my secret=x",
            "ssh-rsa AAAAB3NzaC1yc2E== me@host",
        ];
        for line in table {
            assert_eq!(mask_secret_line(line), MASK, "{line:?}");
        }
    }
}
