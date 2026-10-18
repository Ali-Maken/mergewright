use std::path::PathBuf;

use mergewright_git::remote::RemoteFailure;
use mergewright_git::run::RunError;

use super::{CloneError, name_from_url};

#[test]
fn name_from_url_table() {
    let named = [
        "https://github.com/org/x.git",
        "https://h/org/x/",
        "https://h/org/x/.git",
        "git@github.com:org/x.git",
        "ssh://git@h:22/org/x.git",
        "file:///tmp/r/x.git",
        "/tmp/x",
        "h:x.git",
        "https://h/org/x.git/ \t",
    ];
    for url in named {
        assert_eq!(name_from_url(url).as_deref(), Some("x"), "{url:?}");
    }
    let unnamed = [
        "https://h/",
        "https://h",
        "",
        "h:",
        "/",
        "https://h/org/..",
        "https://h/org/.",
        "https://h/org/x\u{7}y",
    ];
    for url in unnamed {
        assert_eq!(name_from_url(url), None, "{url:?}");
    }
}

#[test]
fn explain_words() {
    let left = Some(PathBuf::from("/code/x"));
    let cases = [
        (CloneError::BadUrl, "can't name a folder from this URL"),
        (
            CloneError::TargetNotEmpty(PathBuf::from("/code/x")),
            "x already exists here",
        ),
        (
            CloneError::Remote {
                failure: RemoteFailure::AuthFailed {
                    target: "https://h".to_owned(),
                },
                stderr_tail: Vec::new(),
                left: None,
            },
            "clone failed: authentication failed for https://h",
        ),
        (
            CloneError::Remote {
                failure: RemoteFailure::Other {
                    first_line: "fatal: nope".to_owned(),
                },
                stderr_tail: Vec::new(),
                left: left.clone(),
            },
            "clone failed: fatal: nope · /code/x was left behind",
        ),
        (
            CloneError::Stopped { left: None },
            "clone stopped · nothing kept",
        ),
        (
            CloneError::Stopped { left: left.clone() },
            "clone stopped · /code/x was left behind",
        ),
        (
            CloneError::Run {
                error: RunError::GitNotFound,
                left: None,
            },
            &format!("clone: {}", RunError::GitNotFound),
        ),
        (
            CloneError::Run {
                error: RunError::GitNotFound,
                left,
            },
            &format!("clone: {} · /code/x was left behind", RunError::GitNotFound),
        ),
    ];
    for (error, words) in cases {
        assert_eq!(error.explain(), words, "{error:?}");
    }
}
