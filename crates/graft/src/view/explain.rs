//! Turning a Mistake Radar `Explanation` (plain text, `graft-ops`) into an
//! `ExplainView` (sanitised, `graft-ui`): the Explain panel's own view model
//! (`m2-details-and-diffs/17`).
//!
//! `Explanation` carries no severity — that comes from the `Finding` it was
//! built from, so `update::detail::open_explain` sets `ExplainView::severity`
//! itself after this module builds the rest.

use graft_core::oid::Oid;
use graft_ops::mistake_radar::explain::Explanation;
use graft_term::sanitize::{SanitizedText, sanitize_line};
use graft_ui::detail::ExplainView;
use graft_ui::graph_screen::{SHA_WIDTH, short_sha};

/// `title`/`paragraphs` sanitised as-is; `sketch` from [`build_sketch`], with
/// commit ids rendered the same short-sha way the SHA header line already
/// does. `severity` is left as a placeholder (`Info`) — the caller
/// (`update::detail::open_explain`) overwrites it from the `Finding`
/// `explain` was built from, which this module never sees.
#[must_use]
pub fn build_explain_view(e: &Explanation) -> ExplainView {
    ExplainView {
        title: sanitize_line(&e.title),
        severity: graft_core::mistake_radar::Severity::Info,
        paragraphs: e.paragraphs.iter().map(|p| sanitize_line(p)).collect(),
        sketch: build_sketch(&e.evidence, |oid| {
            short_sha(&oid.to_string(), usize::from(SHA_WIDTH))
        }),
    }
}

/// One line per evidence entry (`"{label}  {short(oid)}"`, sanitised), a
/// connector line between each consecutive pair alternating `──╮`/`──╯`
/// (DESIGN §1.8's mini-graph shape). For the foxtrot fixture's three
/// evidence entries this is exactly 5 lines: label, connector, label,
/// connector, label.
#[must_use]
pub fn build_sketch(
    evidence: &[(Oid, String)],
    short: impl Fn(Oid) -> String,
) -> Vec<SanitizedText> {
    let mut lines = Vec::with_capacity(evidence.len().saturating_mul(2).saturating_sub(1));
    for (index, (oid, label)) in evidence.iter().enumerate() {
        if index > 0 {
            let connector = if index % 2 == 1 {
                "        \u{2500}\u{2500}\u{256e}"
            } else {
                "        \u{2500}\u{2500}\u{256f}"
            };
            lines.push(sanitize_line(connector));
        }
        lines.push(sanitize_line(&format!("{label}  {}", short(*oid))));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; 20]).unwrap()
    }

    fn short(id: Oid) -> String {
        id.to_string()[..7].to_owned()
    }

    #[test]
    fn sketch_alternates_connectors_between_labels() {
        let evidence = vec![
            (oid(1), "[main] was here".to_string()),
            (oid(2), "this merge".to_string()),
            (oid(3), "first parent".to_string()),
        ];
        let sketch = build_sketch(&evidence, short);
        assert_eq!(sketch.len(), 5);
        assert!(sketch[0].as_str().starts_with("[main] was here  "));
        assert_eq!(sketch[1].as_str(), "        \u{2500}\u{2500}\u{256e}");
        assert!(sketch[2].as_str().starts_with("this merge  "));
        assert_eq!(sketch[3].as_str(), "        \u{2500}\u{2500}\u{256f}");
        assert!(sketch[4].as_str().starts_with("first parent  "));
    }

    #[test]
    fn sketch_of_one_entry_has_no_connector() {
        let evidence = vec![(oid(1), "only".to_string())];
        let sketch = build_sketch(&evidence, short);
        assert_eq!(sketch.len(), 1);
    }

    #[test]
    fn view_carries_title_and_paragraphs_sanitised() {
        let explanation = Explanation {
            title: "FOXTROT MERGE".to_string(),
            paragraphs: vec!["one".to_string(), "two".to_string()],
            evidence: vec![
                (oid(1), "[main] was here".to_string()),
                (oid(2), "this merge".to_string()),
                (oid(3), "first parent".to_string()),
            ],
            highlight: vec![oid(2), oid(3), oid(1)],
        };
        let view = build_explain_view(&explanation);
        assert_eq!(view.title.as_str(), "FOXTROT MERGE");
        assert_eq!(view.paragraphs.len(), 2);
        assert_eq!(view.sketch.len(), 5);
    }
}
