//! `.mailmap` resolution: two identities that map to one canonical author.

use super::*;

fn identity(name: &str, email: &str) -> Identity {
    Identity {
        name: name.as_bytes().to_vec(),
        email: email.as_bytes().to_vec(),
        when: graft_core::commit::Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    }
}

#[test]
fn two_identities_resolve_to_one_canonical_author() {
    let mailmap = Mailmap::from_bytes(b"Rui M <reza@example.com> <reza.old@example.invalid>\n");

    let work_address = identity("Reza Old", "reza.old@example.invalid");
    let (name, email) = mailmap.resolve(&work_address);

    assert_eq!(name, b"Rui M");
    assert_eq!(email, b"reza@example.com");

    // The canonical identity itself resolves to the same place, so both
    // commits draw the same chip in the same colour.
    let canonical = identity("Rui M", "reza@example.com");
    assert_eq!(mailmap.resolve(&canonical), (name, email));
}

#[test]
fn an_identity_the_mailmap_says_nothing_about_resolves_to_itself() {
    let mailmap = Mailmap::from_bytes(b"Rui M <reza@example.com> <reza.old@example.invalid>\n");

    let stranger = identity("Ada K", "ada@example.com");
    let (name, email) = mailmap.resolve(&stranger);

    assert_eq!(name, b"Ada K");
    assert_eq!(email, b"ada@example.com");
}

#[test]
fn an_empty_mailmap_resolves_everything_to_itself() {
    let mailmap = Mailmap::default();
    let identity = identity("Ada K", "ada@example.com");

    assert_eq!(
        mailmap.resolve(&identity),
        (b"Ada K".to_vec(), b"ada@example.com".to_vec())
    );
}
