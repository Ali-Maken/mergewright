//! A blame place's own questions: its rows, its matches, its ignore line and
//! the view handed to the picture.

use graft_ui::blame::{BlameBody, BlameView, FindTitle, IgnoreLine};

use super::{BlameLoad, BlameState};

impl BlameState {
    /// How many rows the body has.
    #[must_use]
    pub fn row_count(&self) -> usize {
        match &self.body {
            BlameLoad::Ready { rows, .. } => rows.len(),
            BlameLoad::Reading { .. } | BlameLoad::Failed(_) => 0,
        }
    }

    /// Rows whose text holds the find, case-folded as the graph search does.
    #[must_use]
    pub fn matches(&self) -> Vec<usize> {
        let (Some(find), BlameLoad::Ready { rows, .. }) = (&self.find, &self.body) else {
            return Vec::new();
        };
        let folded = find.to_lowercase();
        rows.iter()
            .enumerate()
            .filter(|(_, row)| row.text.to_lowercase().contains(&folded))
            .map(|(at, _)| at)
            .collect()
    }

    /// The ignore-revs line, when the repository lists revisions.
    #[must_use]
    pub fn ignore_line(&self) -> Option<IgnoreLine> {
        let BlameLoad::Ready { blame, .. } = &self.body else {
            return None;
        };
        let found = blame.ignore_revs.as_ref()?;
        Some(IgnoreLine {
            count: found.revisions,
            applied: found.applied,
        })
    }

    /// The picture's view of this place, handed to `draw`; `path` is the
    /// sanitised path, which the view borrows.
    pub fn view<R>(&self, path: &str, now: i64, draw: impl FnOnce(&BlameView) -> R) -> R {
        let matches = self.matches();
        let find = self.find.as_deref().map(|query| FindTitle {
            query,
            at: matches
                .iter()
                .filter(|&&at| at <= self.cursor)
                .count()
                .max(1),
            of: matches.len(),
        });
        let body = match &self.body {
            BlameLoad::Reading { .. } => BlameBody::Reading,
            BlameLoad::Failed(words) => BlameBody::Failed(words),
            BlameLoad::Ready { rows, .. } => BlameBody::Lines {
                rows,
                cursor: self.cursor,
                top: self.top,
            },
        };
        let view = BlameView {
            path,
            rev: self.title.clone(),
            body,
            ignore: self.ignore_line(),
            find,
            heatmap: self.heatmap,
            now,
        };
        draw(&view)
    }
}
