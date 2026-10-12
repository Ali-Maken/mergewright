use super::*;

// Expected spellings are literals from ADR-0057 (§2, and the Amendment of
// 2026-10-11 §1), never rebuilt from the table: these tests are the one
// place a generation's names are written out by hand.

#[test]
fn every_generation_spells_its_names() {
    use Generation::{Graft, Mergewright, Tributary};
    let rows = [
        (
            Graft,
            "graft",
            "Graft",
            "graft",
            "GRAFT_",
            "refs/graft/",
            "Graft-Link:",
            false,
        ),
        (
            Tributary,
            "tributary",
            "Tributary",
            "trib",
            "TRIB_",
            "refs/tributary/",
            "Tributary-Link:",
            true,
        ),
        (
            Mergewright,
            "mergewright",
            "Mergewright",
            "mwr",
            "MWR_",
            "refs/mergewright/",
            "Mergewright-Link:",
            true,
        ),
    ];
    for (generation, dir, title, command, env, prefix, trailer, marker) in rows {
        assert_eq!(generation.dir(), dir, "{generation:?}");
        assert_eq!(generation.title(), title, "{generation:?}");
        assert_eq!(generation.command(), command, "{generation:?}");
        assert_eq!(generation.env_prefix(), env, "{generation:?}");
        assert_eq!(generation.ref_prefix(), prefix, "{generation:?}");
        assert_eq!(generation.link_trailer(), trailer, "{generation:?}");
        assert_eq!(generation.has_marker(), marker, "{generation:?}");
    }
    for (generation, email) in [
        (Graft, "graft@backup"),
        (Tributary, "tributary@backup"),
        (Mergewright, "mergewright@backup"),
    ] {
        assert_eq!(generation.backup_email(), email, "{generation:?}");
    }
}

#[test]
fn generations_are_oldest_first_and_current_is_last_reachable() {
    use Generation::{Graft, Mergewright, Tributary};
    assert_eq!(Generation::GENERATIONS, [Graft, Tributary, Mergewright]);
    assert!(
        Generation::GENERATIONS
            .windows(2)
            .all(|pair| pair[0] < pair[1]),
        "the derived order is the release order"
    );
    // Ticket 48 Part B: Mergewright is in force, and it is the last row.
    assert_eq!(Generation::CURRENT, Mergewright);
    assert_eq!(Generation::GENERATIONS.last(), Some(&Generation::CURRENT));
    assert!(Generation::GENERATIONS.contains(&Generation::CURRENT));

    assert_eq!(Graft.older().collect::<Vec<_>>(), []);
    assert_eq!(Tributary.older().collect::<Vec<_>>(), [Graft]);
    assert_eq!(
        Mergewright.older().collect::<Vec<_>>(),
        [Tributary, Graft],
        "newest first"
    );
}

#[test]
fn each_spelling_reads_back_its_generation() {
    use Generation::{Graft, Mergewright, Tributary};
    for (generation, dir, title) in [
        (Graft, "graft", "Graft"),
        (Tributary, "tributary", "Tributary"),
        (Mergewright, "mergewright", "Mergewright"),
    ] {
        let reference = format!("refs/{dir}/backup/main/01J0");
        assert_eq!(Generation::of_ref(&reference), Some(generation));

        let trailer = format!("{title}-Link: step");
        assert_eq!(Generation::of_trailer(&trailer), Some(generation));

        let tagged = format!("{dir}: your edits, set aside for x");
        assert_eq!(Generation::of_message(&tagged), Some(generation));
        let spaced = format!("{dir} 01J0 stage");
        assert_eq!(Generation::of_message(&spaced), Some(generation));
    }

    // Look-alikes that belong to the user, not to any generation.
    for name in ["refs/heads/graft", "refs/graftx/y", "refs/mergewrightx/y"] {
        assert_eq!(Generation::of_ref(name), None, "{name}");
    }
    for line in ["Graft-Linked: x", "graft-link: step", "Signed-off-by: a"] {
        assert_eq!(Generation::of_trailer(line), None, "{line}");
    }
    for message in ["grafting: x", "graftx y", "Fix graft: x", "graft", ""] {
        assert_eq!(Generation::of_message(message), None, "{message:?}");
    }
}
