//! The history place drawn: the view handed to the picture, its title and header.

#[allow(clippy::wildcard_imports)]
use super::*;

impl HistoryState {
    /// The picture's view of this place, handed to `draw`.
    pub fn view<R>(&self, now: i64, theme: &Theme, draw: impl FnOnce(&HistoryView) -> R) -> R {
        let texts: Vec<RowText> = self.rows().iter().map(RowText::of).collect();
        let rows: Vec<HistoryRow> = self
            .rows()
            .iter()
            .zip(&texts)
            .map(|(row, text)| HistoryRow {
                sha7: &text.sha7,
                subject: &text.subject,
                who: &text.who,
                when: row.when,
                added: row.added,
                removed: row.removed,
                was: text.was.as_deref(),
            })
            .collect();
        let title = self.title();
        let end = match &self.trace {
            TraceLoad::Ready { end, .. } => Some(end_words(end)),
            TraceLoad::Tracing { .. } | TraceLoad::Failed(_) => None,
        };
        let failed = match &self.trace {
            TraceLoad::Failed(words) => Some(sanitize_line(words)),
            TraceLoad::Tracing { .. } | TraceLoad::Ready { .. } => None,
        };
        let list = match &failed {
            Some(words) => HistoryList::Failed(words.as_str()),
            None => HistoryList::Rows {
                rows: &rows,
                selected: self.selected,
                top: self.list_top,
                end: end.as_deref(),
            },
        };
        let header = self.header(now);
        let file_failed = self.file_failed_words();
        let sizes_failed = self.sizes_failed_words();
        let file = self.file_pane(&header, file_failed.as_deref(), sizes_failed.as_deref());
        let view = HistoryView {
            title: &title,
            tracing: matches!(self.trace, TraceLoad::Tracing { .. }),
            list,
            file,
            focus: self.focus,
            now,
            theme,
        };
        draw(&view)
    }

    /// `couldn't read <path> at <sha7>: <why>` when the file's read failed.
    fn file_failed_words(&self) -> Option<String> {
        match &self.file {
            FileLoad::Failed { commit, words } => Some(format!(
                "couldn't read {} at {}: {}",
                sanitize_path(self.selected_row().map_or(&self.path, |row| &row.path)).as_str(),
                short(*commit),
                sanitize_line(words).as_str()
            )),
            FileLoad::Idle | FileLoad::Reading { .. } | FileLoad::Ready { .. } => None,
        }
    }

    /// A binary file's size read, when it failed: its words, sanitised. A
    /// binary answer with no size read at all is a wiring bug (the job always
    /// reads them) and says so rather than draw `none → none`.
    fn sizes_failed_words(&self) -> Option<String> {
        match &self.file {
            FileLoad::Ready { view, sizes, .. } if view.binary.is_some() => match sizes {
                Some(Ok(_)) => None,
                Some(Err(words)) => Some(sanitize_line(words).as_str().to_owned()),
                None => Some(SIZES_NOT_READ.to_owned()),
            },
            FileLoad::Idle
            | FileLoad::Reading { .. }
            | FileLoad::Failed { .. }
            | FileLoad::Ready { .. } => None,
        }
    }

    /// The file pane's view: what the trace and the file's read allow.
    fn file_pane<'a>(
        &'a self,
        header: &'a str,
        failed: Option<&'a str>,
        sizes_failed: Option<&'a str>,
    ) -> FilePane<'a> {
        match (&self.trace, &self.file) {
            (TraceLoad::Tracing { .. } | TraceLoad::Failed(_), _) => FilePane::Empty,
            (TraceLoad::Ready { .. }, FileLoad::Idle) => FilePane::NoChange,
            (TraceLoad::Ready { .. }, FileLoad::Reading { .. }) => FilePane::Reading { header },
            (TraceLoad::Ready { .. }, FileLoad::Failed { .. }) => {
                FilePane::Failed(failed.unwrap_or_default())
            }
            (TraceLoad::Ready { .. }, FileLoad::Ready { view, sizes, .. })
                if view.binary.is_some() =>
            {
                let (old, new) = match sizes {
                    Some(Ok(sizes)) => (sizes.old, sizes.new),
                    Some(Err(_)) | None => (None, None),
                };
                FilePane::Binary {
                    header,
                    old,
                    new,
                    sizes_failed,
                }
            }
            (TraceLoad::Ready { .. }, FileLoad::Ready { view, .. }) => FilePane::Lines {
                header,
                hunks: &view.hunks,
                top: self.file_top,
                range: self.range_at().map(|(start, end)| start..=end),
                side_by_side: self.side_by_side,
                word_tint: true,
            },
        }
    }

    /// The title's words after `HISTORY · `: the path, and which lines.
    #[must_use]
    pub fn title(&self) -> String {
        let path = sanitize_path(&self.path);
        match &self.range {
            None => path.as_str().to_owned(),
            Some(range) => format!("{} \u{b7} {}", path.as_str(), range_words(range)),
        }
    }

    /// The file pane's header: `sha7 · Name · age · subject · this file +a −r`
    /// (no counts for a binary file).
    fn header(&self, now: i64) -> String {
        let Some(row) = self.selected_row() else {
            return String::new();
        };
        let mut header = format!(
            "{} \u{b7} {} \u{b7} {} \u{b7} {}",
            short(row.id),
            sanitize_line(&row.author).as_str(),
            relative_date_long(row.when, now),
            sanitize_line(&row.subject).as_str(),
        );
        // A binary file has no lines to count: `+0 −0` beside its size
        // change would read as "no change" (review Minor 8).
        if let Some(view) = self.file_view().filter(|view| view.binary.is_none()) {
            let lines = file_rows(&view.hunks);
            let added = lines
                .iter()
                .filter(|line| line.kind == LineKindView::Added)
                .count();
            let removed = lines
                .iter()
                .filter(|line| line.kind == LineKindView::Removed)
                .count();
            header.push_str(&format!(" \u{b7} this file +{added} \u{2212}{removed}"));
        }
        header
    }
}
/// Screen rows commits `top..=last` take, a `↳ was` row each for a rename.
pub(super) fn drawn_rows(rows: &[TraceCommit], top: usize, last: usize) -> usize {
    rows.iter()
        .enumerate()
        .skip(top)
        .take_while(|(at, _)| *at <= last)
        .map(|(_, row)| 1 + usize::from(row.renamed_from.is_some()))
        .sum()
}

/// A row's sanitised words, owned for the view to borrow.
struct RowText {
    sha7: String,
    subject: String,
    who: String,
    was: Option<String>,
}

impl RowText {
    fn of(row: &TraceCommit) -> RowText {
        RowText {
            sha7: short(row.id),
            subject: sanitize_line(&row.subject).as_str().to_owned(),
            who: sanitize_line(&row.author).as_str().to_owned(),
            was: row
                .renamed_from
                .as_ref()
                .map(|was| sanitize_path(was).as_str().to_owned()),
        }
    }
}

/// `fn refresh_token, lines 18–29` / `lines 22–24 · the block blame showed`.
fn range_words(range: &LineRange) -> String {
    let lines = |start: u32, end: u32| {
        if start == end {
            format!("line {start}")
        } else {
            format!("lines {start}\u{2013}{end}")
        }
    };
    match range {
        LineRange::Function { name, start, end } => {
            format!("{}, {}", function_name(name), lines(*start, *end))
        }
        LineRange::Block { start, end } => {
            format!("{} \u{b7} the block blame showed", lines(*start, *end))
        }
    }
}

/// The function's name as the title says it, from git's header line: the
/// word before the first `(`, with the keyword before it when there is one
/// (`pub fn refresh_token(…` → `fn refresh_token`). A header with no `(`
/// is kept whole.
fn function_name(header: &str) -> String {
    const KEYWORDS: [&str; 6] = ["fn", "def", "func", "function", "sub", "class"];
    let header = sanitize_line(header);
    let header = header.as_str();
    let Some((before, _)) = header.split_once('(') else {
        return header.trim().to_owned();
    };
    let words: Vec<&str> = before.split_whitespace().collect();
    match words.as_slice() {
        [.., keyword, name] if KEYWORDS.contains(keyword) => format!("{keyword} {name}"),
        [.., name] => (*name).to_owned(),
        [] => header.trim().to_owned(),
    }
}

/// The end row's words (H1).
fn end_words(end: &TraceEnd) -> String {
    match end {
        TraceEnd::Start { added_by } => {
            format!("the file starts here ({} added it)", short(*added_by))
        }
        TraceEnd::GitStops(reason) => {
            format!("git stops tracing here: {}", sanitize_line(reason).as_str())
        }
        TraceEnd::Capped { shown } => format!("{} shown \u{b7} older not traced", grouped(*shown)),
        // A stopped trace is a cancelled job: its answer never lands. Said
        // plainly if one ever does, rather than drawing nothing.
        TraceEnd::Stopped => "git stops tracing here: stopped".to_owned(),
    }
}

/// `1 000`: thousands set apart by a space, as H1 writes the cap.
fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(digit);
    }
    out
}
