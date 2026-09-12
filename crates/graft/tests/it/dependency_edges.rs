//! Locks the workspace's internal dependency edges to ARCHITECTURE.md.
//!
//! Cargo rejects dependency *cycles*, but a stray acyclic edge — `graft-ui`
//! taking `graft-git` — compiles cleanly and breaks the layering anyway. This
//! test reads `cargo metadata` and asserts the real edge set matches the crate
//! table in ARCHITECTURE.md exactly, so erosion fails the fast tier instead of
//! waiting for a reviewer to spot it (DESIGN.md §8.5, ADR-0006).
//!
//! Dev-dependencies are deliberately excluded. A test-only helper — `graft-ui`
//! reaching for a `graft-git` fixture builder in its own tests — never ships in
//! the binary and so can't erode the layering. Normal and build dependencies
//! both do ship, so both count. `graft-fixtures` is the standing example: it
//! is a dev-dependency everywhere it is used, so it adds no edge here
//! (ARCHITECTURE.md, "`fixtures`: test-only, outside the table").

use std::collections::BTreeSet;
use std::process::Command;

use serde_json::Value;

/// Every internal dependency ARCHITECTURE.md's crate table declares, as
/// `(dependent, dependency)` package names.
///
/// Adding a crate or an allowed edge means updating this test in the same
/// commit (ADR-0006).
const EXPECTED_EDGES: &[(&str, &str)] = &[
    // `graph`, `git` and `term` are siblings over `core`, and may not see
    // each other.
    ("graft-graph", "graft-core"),
    ("graft-git", "graft-core"),
    ("graft-term", "graft-core"),
    ("graft-ops", "graft-core"),
    ("graft-ops", "graft-git"),
    // `ui` renders from view models `app` assembles; it never sees `git` or
    // `ops`.
    ("graft-ui", "graft-core"),
    ("graft-ui", "graft-graph"),
    ("graft-ui", "graft-term"),
    // `graft` is `app`: the only crate allowed to see every other crate.
    ("graft", "graft-core"),
    ("graft", "graft-graph"),
    ("graft", "graft-git"),
    ("graft", "graft-ops"),
    ("graft", "graft-term"),
    ("graft", "graft-ui"),
];

type Edge = (String, String);

#[test]
fn internal_dependency_edges_match_architecture_md() {
    let actual = internal_edges();
    let expected: BTreeSet<Edge> = EXPECTED_EDGES
        .iter()
        .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
        .collect();

    if actual == expected {
        return;
    }

    panic!(
        "internal dependency edges do not match ARCHITECTURE.md's crate table.\n\
         \n\
         Not allowed (declared in a Cargo.toml, absent from the table):\n{}\n\
         Missing (in the table, absent from every Cargo.toml):\n{}\n\
         \n\
         Either drop the dependency, or — if the edge is intended — update\n\
         ARCHITECTURE.md and EXPECTED_EDGES in this same commit (ADR-0006).",
        render(actual.difference(&expected)),
        render(expected.difference(&actual)),
    );
}

/// The workspace's own `(dependent, dependency)` edges, as Cargo sees them.
fn internal_edges() -> BTreeSet<Edge> {
    let metadata = cargo_metadata();
    let packages = array(&metadata, "packages");

    // `--no-deps` means every package listed is a workspace member.
    let members: BTreeSet<&str> = packages.iter().map(package_name).collect();

    let mut edges = BTreeSet::new();
    for package in packages {
        let from = package_name(package);
        for dependency in array(package, "dependencies") {
            if !ships_in_the_build(dependency) {
                continue;
            }
            let to = string(dependency, "name");
            if members.contains(to) {
                edges.insert((from.to_owned(), to.to_owned()));
            }
        }
    }
    edges
}

/// Normal and build dependencies end up in what Graft ships; dev-dependencies
/// exist only while tests run. `kind` is JSON `null` for a normal dependency,
/// `"build"` or `"dev"` otherwise.
fn ships_in_the_build(dependency: &Value) -> bool {
    match dependency.get("kind") {
        None | Some(Value::Null) => true,
        Some(Value::String(kind)) => kind == "build",
        Some(kind) => panic!("cargo metadata: unexpected dependency kind {kind}"),
    }
}

fn cargo_metadata() -> Value {
    // `--no-deps` keeps this to the workspace's own manifests: no registry
    // resolution, so the test needs no network and runs in milliseconds.
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
            concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
        ])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => panic!("could not run `cargo metadata`: {error}"),
    };
    assert!(
        output.status.success(),
        "`cargo metadata` failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    match serde_json::from_slice(&output.stdout) {
        Ok(value) => value,
        Err(error) => panic!("could not parse `cargo metadata` output as JSON: {error}"),
    }
}

fn array<'a>(value: &'a Value, field: &str) -> &'a [Value] {
    match value.get(field).and_then(Value::as_array) {
        Some(array) => array,
        None => panic!("cargo metadata: expected array field `{field}`"),
    }
}

fn string<'a>(value: &'a Value, field: &str) -> &'a str {
    match value.get(field).and_then(Value::as_str) {
        Some(text) => text,
        None => panic!("cargo metadata: expected string field `{field}`"),
    }
}

fn package_name(package: &Value) -> &str {
    string(package, "name")
}

/// One `from -> to` line per edge, for a failure message that reads as a diff.
fn render<'a>(edges: impl Iterator<Item = &'a Edge>) -> String {
    let mut lines: Vec<String> = edges
        .map(|(from, to)| format!("  {from} -> {to}"))
        .collect();
    if lines.is_empty() {
        lines.push("  (none)".to_owned());
    }
    lines.join("\n")
}
