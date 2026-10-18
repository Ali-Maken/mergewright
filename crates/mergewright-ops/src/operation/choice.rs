//! What goes in a commit before he picks anything (ease-pass/04, note C4–C7):
//! nothing staged means every change goes in, bar a new file that looks like
//! secrets; something staged means only what is staged.

use std::path::{Path, PathBuf};

use mergewright_git::status::WorkingTreeStatus;

use super::Part;

/// Why a file starts out of the commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutWhy {
    /// A new file whose name says keys or credentials ([`looks_like_secrets`]).
    LooksLikeSecrets,
    /// Something else is staged and this is not.
    NotStaged,
}

/// One changed file and whether it starts in the commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChoice {
    /// The path as the status lists it.
    pub path: PathBuf,
    /// It starts in.
    pub goes_in: bool,
    /// What goes in when it does: `StagedPart` for a staged file, else `Whole`.
    pub part: Part,
    /// Why it starts out; `None` exactly when `goes_in`.
    pub why_out: Option<OutWhy>,
}

/// Nothing staged → every change goes in (`Whole`) except new untracked files
/// where [`looks_like_secrets`]; something staged → only staged files
/// (`StagedPart`), the rest out as `NotStaged`. A partially staged file
/// (`staged` and `unstaged` both `Some`) counts as staged. Unmerged paths are
/// left out: no commit runs while one is open.
#[must_use]
pub fn first_choice(status: &WorkingTreeStatus) -> Vec<FileChoice> {
    let anything_staged = status.staged().next().is_some();
    status
        .entries
        .iter()
        .filter(|entry| !entry.unmerged)
        .map(|entry| {
            let (part, why_out) = if anything_staged {
                if entry.is_staged() {
                    (Part::StagedPart, None)
                } else {
                    (Part::Whole, Some(OutWhy::NotStaged))
                }
            } else if entry.untracked && looks_like_secrets(&entry.path) {
                (Part::Whole, Some(OutWhy::LooksLikeSecrets))
            } else {
                (Part::Whole, None)
            };
            FileChoice {
                path: entry.path.clone(),
                goes_in: why_out.is_none(),
                part,
                why_out,
            }
        })
        .collect()
}

/// Names that are secrets whole.
const EXACT: &[&str] = &[
    ".env",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    ".npmrc",
    ".pypirc",
    ".netrc",
    "credentials.json",
    ".htpasswd",
];
/// Endings that are keys and keystores.
const ENDINGS: &[&str] = &[".pem", ".key", ".p12", ".pfx", ".keystore", ".jks"];
/// `.env.<this>` is a template, not secrets.
const SAFE_ENV: &[&str] = &["example", "sample", "template"];

/// Case-insensitive basename: `.env`, `.env.*` (not `.example`/`.sample`/
/// `.template`), `*.pem`, `*.key`, `*.p12`, `*.pfx`, `id_rsa`/`id_dsa`/
/// `id_ecdsa`/`id_ed25519` (not `.pub`), `.npmrc`, `.pypirc`, `.netrc`,
/// `credentials.json`, `*.keystore`, `*.jks`, `secrets.*`, `.htpasswd`.
#[must_use]
pub fn looks_like_secrets(path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };
    let name = name.to_string_lossy().to_lowercase();
    if EXACT.contains(&name.as_str()) || ENDINGS.iter().any(|end| name.ends_with(end)) {
        return true;
    }
    if let Some(rest) = name.strip_prefix(".env.") {
        return !SAFE_ENV.contains(&rest);
    }
    name.starts_with("secrets.")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use mergewright_git::status::{Change, StatusEntry, WorkingTreeStatus};

    use super::{FileChoice, OutWhy, first_choice, looks_like_secrets};
    use crate::operation::Part;

    fn entry(path: &str) -> StatusEntry {
        StatusEntry {
            path: PathBuf::from(path),
            original_path: None,
            staged: None,
            unstaged: None,
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }
    }

    fn modified(path: &str) -> StatusEntry {
        StatusEntry {
            unstaged: Some(Change::Modified),
            ..entry(path)
        }
    }

    fn staged(path: &str) -> StatusEntry {
        StatusEntry {
            staged: Some(Change::Modified),
            ..entry(path)
        }
    }

    fn new(path: &str) -> StatusEntry {
        StatusEntry {
            untracked: true,
            ..entry(path)
        }
    }

    fn status(entries: Vec<StatusEntry>) -> WorkingTreeStatus {
        WorkingTreeStatus { entries }
    }

    fn chosen(path: &str, goes_in: bool, part: Part, why_out: Option<OutWhy>) -> FileChoice {
        FileChoice {
            path: PathBuf::from(path),
            goes_in,
            part,
            why_out,
        }
    }

    #[test]
    fn nothing_staged_puts_every_change_in_but_a_new_secrets_file() {
        let got = first_choice(&status(vec![
            modified("a.rs"),
            modified("b.rs"),
            modified("c.rs"),
            new(".env"),
        ]));
        assert_eq!(
            got,
            vec![
                chosen("a.rs", true, Part::Whole, None),
                chosen("b.rs", true, Part::Whole, None),
                chosen("c.rs", true, Part::Whole, None),
                chosen(".env", false, Part::Whole, Some(OutWhy::LooksLikeSecrets)),
            ]
        );
    }

    #[test]
    fn something_staged_puts_only_staged_files_in() {
        let got = first_choice(&status(vec![staged("a.rs"), modified("b.rs"), new("c.rs")]));
        assert_eq!(
            got,
            vec![
                chosen("a.rs", true, Part::StagedPart, None),
                chosen("b.rs", false, Part::Whole, Some(OutWhy::NotStaged)),
                chosen("c.rs", false, Part::Whole, Some(OutWhy::NotStaged)),
            ]
        );
    }

    #[test]
    fn a_partially_staged_file_counts_as_staged() {
        let partial = StatusEntry {
            staged: Some(Change::Modified),
            unstaged: Some(Change::Modified),
            ..entry("a.rs")
        };
        let got = first_choice(&status(vec![partial, modified("b.rs")]));
        assert_eq!(
            got,
            vec![
                chosen("a.rs", true, Part::StagedPart, None),
                chosen("b.rs", false, Part::Whole, Some(OutWhy::NotStaged)),
            ]
        );
    }

    #[test]
    fn a_tracked_secrets_file_is_not_held_back() {
        // Only a new file is held back: a tracked one is already committed.
        let got = first_choice(&status(vec![modified(".env")]));
        assert_eq!(got, vec![chosen(".env", true, Part::Whole, None)]);
    }

    #[test]
    fn looks_like_secrets_table() {
        let table = [
            (".env", true),
            (".env.local", true),
            (".env.production", true),
            (".env.example", false),
            (".env.sample", false),
            (".env.template", false),
            (".ENV.EXAMPLE", false),
            ("config/server.pem", true),
            ("tls.key", true),
            ("cert.p12", true),
            ("cert.PFX", true),
            ("id_rsa", true),
            ("ID_RSA", true),
            ("home/.ssh/id_ed25519", true),
            ("id_ecdsa", true),
            ("id_dsa", true),
            ("id_rsa.pub", false),
            ("id_ed25519.pub", false),
            (".npmrc", true),
            (".pypirc", true),
            (".netrc", true),
            ("credentials.json", true),
            ("release.keystore", true),
            ("app.jks", true),
            ("secrets.yaml", true),
            ("Secrets.json", true),
            (".htpasswd", true),
            ("src/main.rs", false),
            ("keys.rs", false),
            ("monkey", false),
            ("environment.md", false),
            ("my-secrets.txt", false),
        ];
        for (path, want) in table {
            assert_eq!(looks_like_secrets(Path::new(path)), want, "{path}");
        }
    }
}
