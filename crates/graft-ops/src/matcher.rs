//! The one text matcher: the branch picker's filter and the command palette
//! both rank with it (m8-polish-and-ship/11, decision 1).
//!
//! It lives in `ops`, not `core` (ADR-0007: core holds shapes, not
//! behaviour; m8/12e D4): `graft-ops` ranks the picker with it and `graft`
//! (the palette) already depends on `graft-ops`, so no new edge.

/// How far a query matches `text`; **lower ranks first**, `None` when it
/// does not match at all.
///
/// Case-insensitive. The query splits on whitespace and every word must
/// occur in `text` as a substring. Per word, tier 0 when a hit starts
/// `text` or follows a character that is not a letter or digit (a segment
/// start: `login` in `sara/login-page`), else tier 1; the score is the sum
/// of the tiers × 1024, plus the first word's hit offset in characters
/// (capped at 1023). An empty query matches everything with score 0.
#[must_use]
pub fn fuzzy_score(query: &str, text: &str) -> Option<u32> {
    let text = text.to_lowercase();
    let mut tiers = 0u32;
    let mut first_offset = None;
    for word in query.split_whitespace() {
        let (tier, offset) = best_hit(&text, &word.to_lowercase())?;
        tiers += tier;
        first_offset.get_or_insert(offset);
    }
    let offset = first_offset.unwrap_or(0).min(1023);
    Some(tiers * 1024 + offset)
}

/// The best hit of `word` in `text` (both lowercased): the first segment
/// start if there is one (tier 0), else the first hit (tier 1); the offset
/// in characters. `None` when `word` does not occur.
fn best_hit(text: &str, word: &str) -> Option<(u32, u32)> {
    let mut first = None;
    for (at, _) in text.match_indices(word) {
        let segment_start = text[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric());
        let offset = u32::try_from(text[..at].chars().count()).unwrap_or(u32::MAX);
        if segment_start {
            return Some((0, offset));
        }
        first.get_or_insert((1, offset));
    }
    first
}

#[cfg(test)]
mod tests {
    use super::fuzzy_score;

    #[test]
    fn the_table() {
        // (query, text, expected)
        let cases: &[(&str, &str, Option<u32>)] = &[
            // Segment start beats inside.
            ("login", "login-old", Some(0)),
            ("login", "sara/login-page", Some(5)),
            ("login", "xlogin", Some(1024 + 1)),
            // A later segment start beats an earlier inside hit.
            ("log", "blog/log", Some(5)),
            // Case-insensitive both ways.
            ("Login", "LOGIN-old", Some(0)),
            // Every word must hit.
            ("theme light", "Theme: light", Some(0)),
            ("theme dark", "Theme: light", None),
            ("merge", "Merge the selected branch", Some(0)),
            ("branch", "Merge the selected branch", Some(19)),
            // Two words, one inside: one tier.
            ("me ight", "Theme: light", Some(1024 + 1024 + 3)),
            // Nothing typed matches everything.
            ("", "anything", Some(0)),
            ("   ", "anything", Some(0)),
            ("zzz", "anything", None),
            // Offset is in characters, not bytes.
            ("b", "ä-b", Some(2)),
        ];
        for (query, text, expected) in cases {
            assert_eq!(
                fuzzy_score(query, text),
                *expected,
                "fuzzy_score({query:?}, {text:?})"
            );
        }
    }

    #[test]
    fn segment_start_ranks_below_inside() {
        let start = fuzzy_score("merge", "graph merge").unwrap();
        let inside = fuzzy_score("erge", "graph merge").unwrap();
        assert!(start < inside);
    }
}
