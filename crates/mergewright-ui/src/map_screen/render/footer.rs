//! The Map's footer: keys, the doorway, then the notes, fitted per status
//! (m8/21b-1 split from `render.rs`).

use super::{
    ActionId, Line, MapScreen, MapStatus, Span, Style, Token, fit_place_footer, foreground,
    place_close, place_footer_text,
};

impl MapScreen<'_> {
    /// The footer at `width`: keys, the `? keys` doorway, then the notes
    /// (` · +N more`, ` · partial`), which describe the view rather than
    /// name a key (m8/11e E2). Fitted by the journal's rule
    /// (`hints::fit_place_footer`): the doorway and notes are reserved, the
    /// keys shed into the rest; below that, `+N more` goes, then `partial`,
    /// then the doorway's ladder. Never a cut clause.
    pub(super) fn footer_line(&self, width: u16) -> Line<'static> {
        let muted = foreground(self.theme, Token::TextMuted);
        let keys = self.keys;
        if let MapStatus::Loading { .. } = self.view.status
            && self.view.esc_cancels
        {
            // A write is running: `esc` stops it, not the build, and `M`
            // alone goes back (m8/59 Major A).
            let (cancels, back) = place_close(keys, ActionId("map.close"), "back", true);
            let clauses: Vec<String> = cancels.into_iter().chain(back).collect();
            return Line::styled(place_footer_text(&clauses, keys, usize::from(width)), muted);
        }
        if let MapStatus::Loading { .. } = self.view.status {
            // A cancellable job, not a prompt: `?` opens the palette here
            // (m8/11e G3), so the doorway follows `esc cancels`.
            let cancel: Vec<String> = keys
                .key_verb(&[ActionId("app.back")], "cancels")
                .into_iter()
                .collect();
            return Line::styled(place_footer_text(&cancel, keys, usize::from(width)), muted);
        }
        let clauses = self.ready_clauses();
        let notes = self.notes(muted);
        let note_words: Vec<String> = notes.iter().map(|(words, _)| words.clone()).collect();
        let fitted = fit_place_footer(&clauses, keys, &note_words, usize::from(width));
        let keys_part = fitted.keys_text(&clauses);
        let mut spans = vec![Span::styled(keys_part, muted)];
        for at in fitted.notes {
            let (words, style) = &notes[at];
            // With nothing ahead of it (no key fits and no doorway is
            // bound), a note drops its leading ` · `: no stray separator.
            let words = if spans.iter().all(|span| span.content.is_empty()) {
                words.trim_start_matches(" · ").to_owned()
            } else {
                words.clone()
            };
            spans.push(Span::styled(words, *style));
        }
        Line::from(spans)
    }

    /// The footer's key clauses outside Loading. Spelled from his keymap
    /// (ticket m8/10): the rows move by `app.down`/`app.up` (`j/k`, the
    /// table's first chords — the Map answers both,
    /// `update::map::shown_key`), `⏎` seeks, and `M` and `esc` both close.
    /// Empty and Failed have no row to select, so no `j/k` there (m8/11e
    /// G2, §2.1 mode rule).
    fn ready_clauses(&self) -> Vec<String> {
        let keys = self.keys;
        let has_rows = matches!(self.view.status, MapStatus::Ready);
        // `M/esc back`, or `esc cancels` first and `M back` last while a
        // write runs (m8/59 Major A).
        let (cancels, back) =
            place_close(keys, ActionId("map.close"), "back", self.view.esc_cancels);
        [
            cancels,
            has_rows
                .then(|| keys.key_verb(&[ActionId("app.down"), ActionId("app.up")], "select"))
                .flatten(),
            keys.key_verb(&[ActionId("map.seek")], "zoom to commits"),
            back,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// The notes after the doorway, in 11e's order: ` · +N more` (muted),
    /// then ` · partial` in Warning ink. Each carries its own separator.
    fn notes(&self, muted: Style) -> Vec<(String, Style)> {
        let mut notes = Vec::new();
        if self.view.more > 0 {
            notes.push((format!(" · +{} more", self.view.more), muted));
        }
        if !self.view.complete {
            notes.push((
                " · partial".to_owned(),
                foreground(self.theme, Token::Warning),
            ));
        }
        notes
    }
}
