use super::*;

#[test]
fn message_tag_is_dir() {
    assert_eq!(MESSAGE_TAG, DIR);
}

#[test]
fn is_own_ref() {
    assert!(Own::is_ref(b"refs/mergewright/backup/main/01J0"));
    for name in [
        &b"refs/heads/mergewright"[..],
        b"refs/mergewrightx/y",
        b"refs/other/backup/x",
        b"refs/stray/backup/x",
        b"refs/mergewright\xff",
    ] {
        assert!(!Own::is_ref(name), "{name:?}");
    }
}

#[test]
fn is_own_message() {
    assert!(Own::is_message("mergewright: your edits, set aside for x"));
    assert!(Own::is_message("mergewright 01J0 stage"));
    for message in [
        "other: x",
        "stray 01J0 stage",
        "mergewrightx y",
        "Fix mergewright: x",
        "mergewright",
        "",
    ] {
        assert!(!Own::is_message(message), "{message:?}");
    }
}

#[test]
fn is_own_trailer() {
    assert!(Own::is_trailer("Mergewright-Link: step"));
    for line in [
        "Other-Link: step",
        "Stray-Link: step",
        "mergewright-link: step",
        "Signed-off-by: a",
    ] {
        assert!(!Own::is_trailer(line), "{line}");
    }
}
