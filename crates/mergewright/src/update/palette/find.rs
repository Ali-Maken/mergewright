//! Finding under `⌃P`: the BRANCHES, TAGS, FILES and COMMITS groups, the
//! reads and jobs that answer them, and the cursor as answers land.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// BRANCHES, TAGS, FILES and COMMITS for `query` (tickets 12, 12b). A group
/// not read yet says `reading…`; a failed read says so in its group (H9);
/// one whose answer is for an older query shows that answer's rows the
/// query still matches, then `searching…`, so every row shown matches what
/// is typed; a current answer shows its best [`GROUP_ROWS`] (or
/// [`WIDE_ROWS`] once widened) and `+N more`. A group with no row is left
/// out.
pub(super) fn find_groups(
    find: &Find,
    query: &str,
    tags_read: bool,
    keymap: &Keymap,
) -> Vec<Group> {
    let mut out = Vec::new();
    for group in [FindGroup::Branches, FindGroup::Tags, FindGroup::Files] {
        let read = match group {
            FindGroup::Branches => read_state(&find.branches, "branches"),
            FindGroup::Files => read_state(&find.files, "files"),
            FindGroup::Tags | FindGroup::Commits => Ok(tags_read),
        };
        let rows = match read {
            Ok(true) => group_rows(find, group, query),
            Ok(false) => vec![placeholder("reading…")],
            Err(words) => vec![placeholder(&words)],
        };
        if !rows.is_empty() {
            out.push((sanitize_line(group.header()), rows));
        }
    }
    let commits = commit_rows(&find.commits, query, keymap);
    if !commits.is_empty() {
        out.push((sanitize_line(FindGroup::Commits.header()), commits));
    }
    out
}

/// A once-per-open read: landed (`true`), not yet (`false`), or its
/// failure as the group's one row, `<what>: <reason>` on one line, as the
/// `C` picker words its failure.
pub(super) fn read_state<T>(read: &ReadOnce<T>, what: &str) -> Result<bool, String> {
    match read {
        ReadOnce::NotAsked | ReadOnce::Reading(_) => Ok(false),
        ReadOnce::Read(_) => Ok(true),
        ReadOnce::Failed(why) => Err(format!("{what}: {}", one_line(why))),
    }
}

/// `why`'s non-blank lines, trimmed, joined by a space.
pub(super) fn one_line(why: &str) -> String {
    let lines: Vec<&str> = why
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    lines.join(" ")
}

/// One read group's rows from the names answer: current, or stale and
/// re-filtered with `searching…` after.
pub(super) fn group_rows(
    find: &Find,
    group: FindGroup,
    query: &str,
) -> Vec<(PaletteRow, PaletteTarget)> {
    let answered = answered_rows(find, group);
    let cap = if find.widened == Some(group) {
        WIDE_ROWS
    } else {
        GROUP_ROWS
    };
    match answered {
        Some((asked, rows, total)) if asked == query => current_rows(rows, total, group, cap),
        stale => {
            let mut rows: Vec<_> = stale
                .map(|(_, rows, _)| rows)
                .unwrap_or_default()
                .into_iter()
                .filter(|(row, _)| fuzzy_score(query, row.label.as_str()).is_some())
                .take(cap)
                .collect();
            rows.push(placeholder("searching…"));
            rows
        }
    }
}

/// The names answer's rows for `group`, best first, with the query it
/// answered and how many matched in all. `None` when no answer carries
/// the group yet.
pub(super) fn answered_rows(find: &Find, group: FindGroup) -> Option<(&str, Rows, usize)> {
    let (asked, hits) = find.names.answer.as_ref()?;
    let refs = |ranked: Option<&Ranked<RefHit>>, kind| {
        ranked.map(|ranked| {
            let rows = ranked.top.iter().map(|hit| hit_row(hit, kind)).collect();
            (rows, ranked.total)
        })
    };
    let (rows, total) = match group {
        FindGroup::Branches => refs(hits.branches.as_ref(), RowKind::Branch),
        FindGroup::Tags => refs(hits.tags.as_ref(), RowKind::Tag),
        FindGroup::Files => {
            let ReadOnce::Read(paths) = &find.files else {
                return None;
            };
            hits.files.as_ref().map(|ranked| {
                let rows = ranked
                    .top
                    .iter()
                    .filter_map(|ix| file_row(paths, *ix))
                    .collect();
                (rows, ranked.total)
            })
        }
        FindGroup::Commits => None,
    }?;
    Some((asked.as_str(), rows, total))
}

/// A current answer's rows: the best `cap`, then `+N more` (widens) or,
/// once widened, `+N more · type more to narrow` (a note).
pub(super) fn current_rows(
    rows: Vec<(PaletteRow, PaletteTarget)>,
    total: usize,
    group: FindGroup,
    cap: usize,
) -> Vec<(PaletteRow, PaletteTarget)> {
    let mut rows: Vec<_> = rows.into_iter().take(cap).collect();
    let more = total.saturating_sub(rows.len());
    if more > 0 {
        if cap >= WIDE_ROWS {
            rows.push(placeholder(&format!("+{more} more · type more to narrow")));
        } else {
            let (row, _) = placeholder(&format!("+{more} more"));
            rows.push((row, PaletteTarget::More(group)));
        }
    }
    rows
}

pub(super) fn hit_row(hit: &RefHit, kind: RowKind) -> (PaletteRow, PaletteTarget) {
    (
        PaletteRow {
            label: sanitize_line(&String::from_utf8_lossy(&hit.name)),
            key: None,
            config: None,
            kind,
        },
        PaletteTarget::Seek(hit.tip),
    )
}

/// A FILES row: the path as git lists it; the widget mutes its directory.
pub(super) fn file_row(paths: &TrackedPaths, ix: u32) -> Option<(PaletteRow, PaletteTarget)> {
    let path = paths.get(ix as usize)?;
    Some((
        PaletteRow {
            label: sanitize_line(&String::from_utf8_lossy(path)),
            key: None,
            config: None,
            kind: RowKind::File,
        },
        PaletteTarget::File(ix),
    ))
}

/// The query changed (a key, a paste): under `⌃P`, the widening reset, the
/// names job in flight cancelled and one asked for the new query; the
/// cursor on the first row that runs.
pub(super) fn query_changed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    palette: &mut PaletteState,
) {
    if palette.mode == PaletteMode::Everything {
        palette.find.widened = None;
        // A space typed or deleted at either end asks nothing new.
        let query = palette.query.text().trim();
        let names_stale = last_asked(&palette.find.names).is_none_or(|asked| asked != query);
        let commits_stale = last_asked(&palette.find.commits).is_none_or(|asked| asked != query);
        if names_stale {
            ask_names(state, commands, palette);
        }
        if commits_stale {
            ask_commits(state, commands, palette);
        }
    }
    palette.cursor = first_runnable(&rows(state, palette));
}

/// The query a job was last asked for: the one in flight, else the one
/// answered.
pub(super) fn last_asked<T>(asked: &Asked<T>) -> Option<&str> {
    match (&asked.pending, &asked.answer) {
        (Some((_, query)), _) | (None, Some((query, _))) => Some(query),
        (None, None) => None,
    }
}

/// Cancel the commits job in flight and, when there is a query, ask one
/// for it (12b). An empty box asks none (P-find-1).
pub(super) fn ask_commits(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    palette: &mut PaletteState,
) {
    if let Some((id, _)) = palette.find.commits.pending.take() {
        commands.push(Command::CancelJob(id));
    }
    let query = palette.query.text().trim().to_owned();
    let Some(parsed) = crate::search::Query::parse(&query) else {
        return;
    };
    let id = next_id(state);
    commands.push(Command::PaletteCommits(id, parsed));
    palette.find.commits.pending = Some((id, query));
}

/// Cancel the names job in flight and, when there is a query and something
/// read to match it against, ask one for the query as it stands.
pub(super) fn ask_names(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    palette: &mut PaletteState,
) {
    if let Some((id, _)) = palette.find.names.pending.take() {
        commands.push(Command::CancelJob(id));
    }
    let query = palette.query.text().trim().to_owned();
    if query.is_empty() {
        return;
    }
    let branches = match &palette.find.branches {
        ReadOnce::Read(read) => Some(Arc::clone(read)),
        ReadOnce::NotAsked | ReadOnce::Reading(_) | ReadOnce::Failed(_) => None,
    };
    let files = match &palette.find.files {
        ReadOnce::Read(read) => Some(Arc::clone(read)),
        ReadOnce::NotAsked | ReadOnce::Reading(_) | ReadOnce::Failed(_) => None,
    };
    let tags = state.refs.clone();
    if branches.is_none() && tags.is_none() && files.is_none() {
        return;
    }
    let id = next_id(state);
    commands.push(Command::PaletteNames(
        id,
        NamesRequest {
            query: Arc::from(query.as_str()),
            branches,
            tags,
            files,
        },
    ));
    palette.find.names.pending = Some((id, query));
}

/// The palette closed: its reads in flight are no one's any more.
pub(super) fn cancel_finds(palette: &PaletteState, commands: &mut Vec<Command>) {
    if let ReadOnce::Reading(id) = palette.find.branches {
        commands.push(Command::CancelJob(id));
    }
    if let ReadOnce::Reading(id) = palette.find.files {
        commands.push(Command::CancelJob(id));
    }
    for (id, _) in [&palette.find.names.pending, &palette.find.commits.pending]
        .into_iter()
        .flatten()
    {
        commands.push(Command::CancelJob(*id));
    }
}

/// `+N more` on `group`: show up to [`WIDE_ROWS`], the cursor on the first
/// row it added.
pub(super) fn widen(state: &AppState, palette: &mut PaletteState, group: FindGroup) {
    palette.find.widened = Some(group);
    let mut flat = 0;
    for (header, rows) in rows(state, palette) {
        if header.as_str() == group.header() {
            palette.cursor = flat + GROUP_ROWS.min(rows.len().saturating_sub(1));
            return;
        }
        flat += rows.len();
    }
}

/// The branch read landed, or failed. Another open's (a stale id) is
/// dropped. Landed: the names job is asked again with the branches.
pub(in crate::update) fn branches_read(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    result: Result<Arc<SwitchRefs>, String>,
) {
    let ours = state
        .palette
        .as_ref()
        .is_some_and(|p| p.find.branches == ReadOnce::Reading(id));
    if !ours {
        return;
    }
    landed(state, |state, palette| {
        palette.find.branches = match result {
            Ok(read) => ReadOnce::Read(read),
            Err(why) => ReadOnce::Failed(why),
        };
        if matches!(palette.find.branches, ReadOnce::Read(_)) {
            ask_names(state, commands, palette);
        }
    });
}

/// The tracked-path read landed, or failed (12b). Another open's (a stale
/// id) is dropped. Landed: the names job is asked again with the paths.
pub(in crate::update) fn tracked_read(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    result: Result<Arc<TrackedPaths>, String>,
) {
    let ours = state
        .palette
        .as_ref()
        .is_some_and(|p| p.find.files == ReadOnce::Reading(id));
    if !ours {
        return;
    }
    landed(state, |state, palette| {
        palette.find.files = match result {
            Ok(read) => ReadOnce::Read(read),
            Err(why) => ReadOnce::Failed(why),
        };
        if matches!(palette.find.files, ReadOnce::Read(_)) {
            ask_names(state, commands, palette);
        }
    });
}

/// The commits job answered (12b). Any but the one in flight (an older
/// query's, a closed palette's) is dropped.
pub(in crate::update) fn commits_answered(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<[CommitHit]>, String>,
) {
    let ours = state
        .palette
        .as_ref()
        .and_then(|p| p.find.commits.pending.as_ref())
        .is_some_and(|(pending, _)| *pending == id);
    if !ours {
        return;
    }
    landed(state, |_, palette| {
        if let Some((_, query)) = palette.find.commits.pending.take() {
            palette.find.commits.answer = Some((query, result));
        }
    });
}

/// The names job answered. Any but the one in flight (an older query's, a
/// closed palette's) is dropped.
pub(in crate::update) fn names_answered(state: &mut AppState, id: JobId, hits: Arc<NameHits>) {
    let ours = state
        .palette
        .as_ref()
        .and_then(|p| p.find.names.pending.as_ref())
        .is_some_and(|(pending, _)| *pending == id);
    if !ours {
        return;
    }
    landed(state, |_, palette| {
        if let Some((_, query)) = palette.find.names.pending.take() {
            palette.find.names.answer = Some((query, hits));
        }
    });
}

/// The refs job answered while `⌃P` has a query: TAGS can be matched now,
/// so the names job is asked again.
pub(in crate::update) fn refs_landed(state: &mut AppState, commands: &mut Vec<Command>) {
    let typed = state
        .palette
        .as_ref()
        .is_some_and(|p| p.mode == PaletteMode::Everything && !p.query.text().trim().is_empty());
    if typed {
        landed(state, |state, palette| ask_names(state, commands, palette));
    }
}
