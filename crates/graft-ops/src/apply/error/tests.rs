use super::*;

fn denied(path: &str) -> RestoreError {
    RestoreError::WriteRaw {
        path: PathBuf::from(path),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    }
}

/// Ruling 2026-10-01 §1: the three wordings, exact. One path names it;
/// more than one names the first that failed; a failed check carries
/// its own words. No `u` clause in any.
#[test]
fn not_as_they_were_says_the_edits_are_back_and_where_the_bytes_are() {
    let one = AfterState::NotAsTheyWere {
        error: denied("src/a.rs"),
        files: 1,
    };
    assert_eq!(
        one.to_string(),
        "your edits are back · src/a.rs not written back as it was: denied · its exact bytes \
         are in the backup"
    );
    let more = AfterState::NotAsTheyWere {
        error: denied("src/b.rs"),
        files: 3,
    };
    assert_eq!(
        more.to_string(),
        "your edits are back · not all written back as they were: src/b.rs: denied · their \
         exact bytes are in the backup"
    );
    let check = AfterState::NotAsTheyWere {
        error: RestoreError::Run(RunError::Unparsable("hash-object printed \"?\"".to_owned())),
        files: 0,
    };
    assert_eq!(
        check.to_string(),
        "your edits are back · not written back as they were: git's output did not parse as \
         expected: hash-object printed \"?\" · their exact bytes are in the backup"
    );
}
