//! The point of this whole crate (see `docs/capabilities/README.md`): every
//! citation in `docs/capabilities/*.md` — a test name, a capture path, an
//! ADR or design note — must check out against the repository on disk.
//!
//! This runs as a normal `cargo nextest run` test, not a script someone has
//! to remember: a citation that rots (a renamed test, a deleted capture)
//! fails the fast tier the same day it rots, by name, file and line.

use std::path::{Path, PathBuf};

/// Walks up from this crate's manifest directory to find the workspace
/// root, the same way `src/bin/gen_capabilities.rs` does.
fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if workspace_manifest(&dir) {
            return dir;
        }
        assert!(
            dir.pop(),
            "could not find the workspace root above {}",
            env!("CARGO_MANIFEST_DIR")
        );
    }
}

fn workspace_manifest(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .is_ok_and(|content| content.contains("[workspace]"))
}

#[test]
fn every_capability_citation_checks_out() {
    let repo_root = repo_root();
    let capabilities = mergewright_capabilities::load(&repo_root)
        .unwrap_or_else(|error| panic!("docs/capabilities/*.md did not parse: {error}"));

    assert!(
        capabilities.len() > 100,
        "expected well over 100 capabilities (the six milestone files have ~124); \
         got {} — docs/capabilities/ probably failed to glob correctly",
        capabilities.len()
    );

    let findings = mergewright_capabilities::check(&repo_root, &capabilities);
    assert!(
        findings.is_empty(),
        "\n{} capability citation(s) do not check out. Each line below names the \
         file, the line of the capability's heading, and the dead citation:\n\n{}\n\n\
         Fix the capability entry (rename the citation, add the missing capture/ADR, \
         or update the test name), or delete the stale claim if it no longer holds.\n",
        findings.len(),
        findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

#[test]
fn every_capability_has_a_well_formed_schema() {
    // `load` itself fails fast on the first malformed `meta` block (missing
    // field, id/heading mismatch, bad status value) — see
    // `mergewright_capabilities::LoadError::MalformedSchema`. A successful load
    // is the schema check; this test exists so a schema regression shows up
    // under its own name rather than folded into the citations test above.
    let repo_root = repo_root();
    mergewright_capabilities::load(&repo_root)
        .unwrap_or_else(|error| panic!("malformed capability schema: {error}"));
}
