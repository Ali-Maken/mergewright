//! A blame place's own questions: its rows, its matches, its ignore line and
//! the view handed to the picture.

use mergewright_term::sanitize::sanitize_line;
use mergewright_ui::blame::{BlameBody, BlameView, FindTitle, IgnoreLine};

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

    /// The final line under the cursor, once the blame is read.
    #[must_use]
    pub fn cursor_line(&self) -> Option<u32> {
        let BlameLoad::Ready { blame, .. } = &self.body else {
            return None;
        };
        blame.lines.get(self.cursor).map(|line| line.final_line)
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

    /// The ignore-revs line, when the repository lists revisions. While a
    /// re-read runs, the last answer's count, worded for the option chosen.
    #[must_use]
    pub fn ignore_line(&self) -> Option<IgnoreLine> {
        match &self.body {
            BlameLoad::Ready { blame, .. } => {
                let found = blame.ignore_revs.as_ref()?;
                Some(IgnoreLine {
                    count: found.revisions,
                    applied: found.applied,
                })
            }
            BlameLoad::Reading { ignore_listed, .. } => ignore_listed.map(|count| IgnoreLine {
                count,
                applied: self.options.ignore_revs,
            }),
            BlameLoad::Failed(_) => None,
        }
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
        // git's words come from outside: neutralised before they are drawn.
        let failed = sanitize_line(match &self.body {
            BlameLoad::Failed(words) => words,
            BlameLoad::Reading { .. } | BlameLoad::Ready { .. } => "",
        });
        let body = match &self.body {
            BlameLoad::Reading { .. } => BlameBody::Reading,
            BlameLoad::Failed(_) => BlameBody::Failed(failed.as_str()),
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
            whitespace_ignored: self.options.ignore_whitespace,
            default_ignored: mergewright_git::blame::BlameOptions::default().ignore_whitespace,
        };
        draw(&view)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mergewright_git::blame::{BlameOptions, Rev};
    use mergewright_ui::blame::BlameRev;

    use super::super::{BlameLoad, BlameState};

    /// 21b-2 item 2: the view's default is mergewright-git's, so a change to
    /// `BlameOptions::default` moves the title with it.
    #[test]
    fn the_view_carries_blames_own_default() {
        let place = BlameState {
            path: PathBuf::from("a.rs"),
            rev: Rev::WorkingTree,
            title: BlameRev::Now,
            options: BlameOptions::default(),
            body: BlameLoad::Failed("no".into()),
            cursor: 0,
            top: 0,
            find: None,
            find_prompt: None,
            heatmap: false,
            last_click: None,
        };
        let default_ignored = place.view("a.rs", 0, |view| view.default_ignored);
        assert_eq!(default_ignored, BlameOptions::default().ignore_whitespace);
    }
}
