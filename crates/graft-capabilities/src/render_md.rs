//! Generates the root `CAPABILITIES.md` from parsed capabilities.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::model::{Capability, Status};

const REGENERATE_COMMAND: &str = "cargo run -p graft-capabilities --bin gen-capabilities";

/// Renders the full `CAPABILITIES.md` document: a generated banner, the
/// status distribution, then every capability grouped by area (not
/// milestone — "what can this thing do" is an area question).
#[must_use]
pub fn render(capabilities: &[Capability]) -> String {
    let mut out = String::new();
    header(&mut out, capabilities);
    by_area(&mut out, capabilities);
    unverified_aspects(&mut out, capabilities);
    out
}

fn header(out: &mut String, capabilities: &[Capability]) {
    let _ = writeln!(out, "# CAPABILITIES.md — what Graft can do");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "**Generated from `docs/capabilities/*.md`. Do not hand-edit this file** — \
         edit the capability's entry in its milestone file under \
         `docs/capabilities/`, then regenerate:"
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "```\n{REGENERATE_COMMAND}\n```");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "The browsable, filterable version of this same data is \
         `docs/generated/capabilities.html` (open it directly, no server \
         needed). The schema, and how to add a capability, is \
         `docs/capabilities/README.md`."
    );
    let _ = writeln!(out);

    distribution(out, capabilities);
}

fn distribution(out: &mut String, capabilities: &[Capability]) {
    let total = capabilities.len();
    let mut counts: BTreeMap<Status, usize> = BTreeMap::new();
    for cap in capabilities {
        *counts.entry(cap.status).or_default() += 1;
    }

    let _ = writeln!(out, "## At a glance");
    let _ = writeln!(out);
    let _ = writeln!(out, "{total} capabilities tracked.");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Status | Count | Share | |");
    let _ = writeln!(out, "|---|---:|---:|---|");
    for status in [
        Status::Works,
        Status::Partial,
        Status::Unverified,
        Status::Broken,
    ] {
        let n = counts.get(&status).copied().unwrap_or(0);
        #[allow(clippy::cast_precision_loss)]
        let pct = if total == 0 {
            0.0
        } else {
            100.0 * n as f64 / total as f64
        };
        let bar_len = n.saturating_mul(40).checked_div(total).unwrap_or(0);
        let bar = "#".repeat(bar_len);
        let _ = writeln!(out, "| {status} | {n} | {pct:.1}% | `{bar}` |");
    }
    let _ = writeln!(out);

    // Orthogonal to the status column above: a `works` or `partial`
    // capability can still carry one named, unverified aspect (usually
    // "how it looks" — captures can't prove colour, CLAUDE.md rule 5). This
    // is not double-counted into the `unverified` row, which is reserved
    // for a capability that is *itself* unverified end to end.
    let carrying_aspect = capabilities
        .iter()
        .filter(|c| c.unverified_aspect.is_some())
        .count();
    let _ = writeln!(
        out,
        "**{carrying_aspect} of the {total}** otherwise-`works`/`partial` capabilities carry \
         at least one explicitly unverified aspect (colour, a real terminal, a real network \
         peer — the sort of thing a captured character grid can never settle). Full list: \
         [§ Unverified aspects](#unverified-aspects)."
    );
    let _ = writeln!(out);
}

fn by_area(out: &mut String, capabilities: &[Capability]) {
    let mut areas: BTreeMap<&str, Vec<&Capability>> = BTreeMap::new();
    for cap in capabilities {
        areas.entry(cap.area.as_str()).or_default().push(cap);
    }

    let _ = writeln!(out, "## By area");
    let _ = writeln!(out);

    for (area, mut caps) in areas {
        caps.sort_by(|a, b| a.id.cmp(&b.id));
        let works = caps.iter().filter(|c| c.status == Status::Works).count();
        let _ = writeln!(
            out,
            "### {area} ({} capabilities, {works} works)",
            caps.len()
        );
        let _ = writeln!(out);
        for cap in caps {
            let anchor = slug(&format!("{}. {}", cap.id, cap.title));
            let marker = if cap.unverified_aspect.is_some() {
                " ⚠"
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "- **{}** {} — `{}`{} — [{}#{}]({}#{})",
                cap.id,
                cap.title,
                cap.status,
                marker,
                cap.source_file.display(),
                anchor,
                cap.source_file.display(),
                anchor,
            );
        }
        let _ = writeln!(out);
    }
}

/// "Show me everything nobody has actually looked at": every capability
/// carrying a `unverified_aspect`, with the exact aspect named — the same
/// data the by-area listing marks with `⚠`, gathered in one place.
fn unverified_aspects(out: &mut String, capabilities: &[Capability]) {
    let mut with_aspect: Vec<&Capability> = capabilities
        .iter()
        .filter(|c| c.unverified_aspect.is_some())
        .collect();
    with_aspect.sort_by(|a, b| a.id.cmp(&b.id));

    let _ = writeln!(out, "## Unverified aspects");
    let _ = writeln!(out);
    if with_aspect.is_empty() {
        let _ = writeln!(out, "None currently.");
        let _ = writeln!(out);
        return;
    }
    for cap in with_aspect {
        let aspect = cap.unverified_aspect.as_deref().unwrap_or_default();
        let _ = writeln!(
            out,
            "- **{}** ({}, `{}`) — {aspect}",
            cap.id, cap.area, cap.status
        );
    }
    let _ = writeln!(out);
}

/// A GitHub-flavoured-markdown-style anchor slug: lowercase, spaces to
/// hyphens, everything else that isn't alphanumeric or a hyphen dropped.
/// Close enough for GitHub and VS Code's own markdown preview to resolve;
/// it doesn't need to be exact for a human scrolling to find the heading
/// nearby.
fn slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
        } else if ch == ' ' || ch == '-' {
            out.push('-');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cap(id: &str, area: &str, status: Status) -> Capability {
        Capability {
            id: id.to_owned(),
            title: "A title".to_owned(),
            area: area.to_owned(),
            milestone: "m9".to_owned(),
            status,
            unverified_aspect: None,
            status_prose: String::new(),
            tests: Vec::new(),
            captures: Vec::new(),
            decided_in: String::new(),
            decisions: Vec::new(),
            source_file: PathBuf::from("docs/capabilities/m9.md"),
            source_line: 1,
        }
    }

    #[test]
    fn groups_by_area_not_milestone() {
        let caps = vec![
            cap("M1-01", "graph", Status::Works),
            cap("M2-01", "graph", Status::Partial),
            cap("M3-01", "undo", Status::Works),
        ];
        let rendered = render(&caps);
        let graph_idx = rendered.find("### graph").unwrap();
        let undo_idx = rendered.find("### undo").unwrap();
        assert!(graph_idx < undo_idx);
        assert!(rendered.contains("2 capabilities"));
        assert!(rendered.contains(REGENERATE_COMMAND));
    }

    #[test]
    fn distribution_counts_every_status() {
        let caps = vec![
            cap("M1-01", "graph", Status::Works),
            cap("M1-02", "graph", Status::Broken),
        ];
        let rendered = render(&caps);
        assert!(rendered.contains("2 capabilities tracked"));
    }

    #[test]
    fn unverified_aspect_is_counted_marked_and_listed() {
        let mut with_aspect = cap("M1-01", "graph", Status::Works);
        with_aspect.unverified_aspect = Some("colour, unverified by eye".to_owned());
        let caps = vec![with_aspect, cap("M1-02", "graph", Status::Works)];
        let rendered = render(&caps);
        assert!(rendered.contains("**1 of the 2**"));
        assert!(rendered.contains("M1-01"));
        assert!(rendered.contains("⚠"));
        assert!(rendered.contains("## Unverified aspects"));
        assert!(rendered.contains("colour, unverified by eye"));
        // The other capability has no aspect and must not be listed there.
        let section_start = rendered.find("## Unverified aspects").unwrap();
        assert!(!rendered[section_start..].contains("M1-02"));
    }
}
