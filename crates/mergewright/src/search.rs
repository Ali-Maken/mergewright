//! Search over history (`/`, m1-the-graph/26): the query a user typed and the
//! pure test of one commit against it, plus how a search job stands.
//!
//! Matching is a case-insensitive literal substring over the raw repository
//! bytes — subject, author name and email, the full message body — or a prefix
//! of the commit id. No regex and no query language. Nothing here draws: text
//! shown for a hit goes through `SanitizedText` like every other repo string.

use std::sync::Arc;

use mergewright_core::commit::Commit;
use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;
use mergewright_git::history::{Order, Walk};

use crate::repos::RepoStore;

/// A parsed search query: the literal text, folded for case-insensitive
/// matching, and whether it could also name a commit by id prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// What the user typed, verbatim (for the prompt and the status line).
    raw: String,
    /// `raw` lowercased; compared against lowercased haystacks.
    folded: String,
    /// `raw` as a lowercase hex prefix, when it is one.
    sha_prefix: Option<String>,
}

impl Query {
    /// Parse a typed query. `None` for an empty (or all-whitespace) query:
    /// there is nothing to search for.
    pub fn parse(raw: &str) -> Option<Query> {
        if raw.trim().is_empty() {
            return None;
        }
        let folded = raw.to_lowercase();
        let sha_prefix = folded
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
            .then(|| folded.clone());
        Some(Query {
            raw: raw.to_owned(),
            folded,
            sha_prefix,
        })
    }

    /// The query as typed.
    pub fn text(&self) -> &str {
        &self.raw
    }

    /// Does this commit match? `subject`, `author` (name and email, however the
    /// caller joins them) and `body` are raw repository bytes, not necessarily
    /// UTF-8.
    pub fn matches(&self, id: Oid, subject: &[u8], author: &[u8], body: &[u8]) -> bool {
        if let Some(prefix) = &self.sha_prefix
            && id.to_string().starts_with(prefix.as_str())
        {
            return true;
        }
        [subject, author, body]
            .into_iter()
            .any(|field| self.found_in(field))
    }

    fn found_in(&self, haystack: &[u8]) -> bool {
        let needle = self.folded.as_bytes();
        if self.folded.is_ascii() {
            // ASCII needle: fold bytes in place, no allocation, and still
            // correct over non-UTF-8 haystacks.
            return haystack
                .windows(needle.len())
                .any(|window| window.eq_ignore_ascii_case(needle));
        }
        match std::str::from_utf8(haystack) {
            Ok(text) => text.to_lowercase().contains(self.folded.as_str()),
            // A non-ASCII needle can only match valid UTF-8 text; a byte
            // comparison is the best a malformed haystack allows.
            Err(_) => haystack
                .windows(needle.len())
                .any(|window| window == needle),
        }
    }
}

/// Where a search job stands. Rendered by state: a running or finished search
/// with no hits is not an error, and only `Failed` reads as one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchState {
    /// Still reading history; `hits` so far, and more may come.
    Running {
        /// Matches found so far.
        hits: usize,
    },
    /// Read every commit; `hits` is final (zero is a result, not a failure).
    Done {
        /// Matches in all of history.
        hits: usize,
    },
    /// Stopped by the user before it finished; `hits` found by then.
    Cancelled {
        /// Matches found before the stop.
        hits: usize,
    },
    /// The walk failed; why.
    Failed(String),
}

impl SearchState {
    /// The status line's words for this state.
    pub fn describe(&self) -> String {
        match self {
            SearchState::Running { hits } => format!("{hits} {} so far", plural(*hits)),
            SearchState::Done { hits: 0 } => "no matches".to_owned(),
            SearchState::Done { hits } => format!("{hits} {}", plural(*hits)),
            SearchState::Cancelled { hits } => {
                format!("search cancelled, {hits} {} found", plural(*hits))
            }
            SearchState::Failed(why) => format!("search failed: {why}"),
        }
    }
}

fn plural(hits: usize) -> &'static str {
    if hits == 1 { "match" } else { "matches" }
}

/// How [`scan`] ended when it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scanned {
    /// Every commit reachable from every ref was read.
    Finished,
    /// Stopped by `stop`, or `found` said nobody is listening.
    Stopped,
}

/// Reads history from every ref, `chunk` commits at a time, handing each
/// chunk's hits to `found` as soon as the chunk is read (a job's work). `found`
/// returns `false` when nobody is listening any more. `stop` is checked before
/// every chunk.
pub fn scan(
    store: &RepoStore,
    query: &Query,
    chunk: usize,
    stop: &dyn Fn() -> bool,
    found: &mut dyn FnMut(Arc<[Oid]>) -> bool,
) -> Result<Scanned, String> {
    walk_matching(store, query, chunk, stop, &mut |hits| {
        hits.is_empty() || found(hits.iter().map(|commit| commit.id).collect())
    })
}

/// One commit the palette's COMMITS group lists: its id and subject line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitHit {
    /// The commit.
    pub id: Oid,
    /// Its subject, raw repository bytes (sanitised when drawn).
    pub subject: Box<[u8]>,
}

/// How many commits [`first_hits`] reads per chunk: small, so a query that
/// hits early stops early, and still one `stop` check per chunk.
const FIRST_HITS_CHUNK: usize = 64;

/// The first `limit` commits [`scan`] would find, in the same walk order
/// (topological from every ref, newest first) — the palette's COMMITS group
/// (m8 12b). Stops reading as soon as `limit` are found; a query with fewer
/// hits walks all of history (a job's work, never the loop's).
///
/// # Errors
/// The walk's own failure, as words.
pub fn first_hits(
    store: &RepoStore,
    query: &Query,
    limit: usize,
    stop: &dyn Fn() -> bool,
) -> Result<Vec<CommitHit>, String> {
    let mut hits = Vec::new();
    if limit == 0 {
        return Ok(hits);
    }
    // `Stopped` and `Finished` both mean "this is all there is": the hits
    // in hand. A cancelled job's answer is dropped by its caller.
    walk_matching(store, query, FIRST_HITS_CHUNK, stop, &mut |found| {
        for commit in found.iter().take(limit - hits.len()) {
            hits.push(CommitHit {
                id: commit.id,
                subject: commit.subject().into(),
            });
        }
        hits.len() < limit
    })?;
    Ok(hits)
}

/// The walk both [`scan`] and [`first_hits`] share: each chunk's matching
/// commits (possibly none) go to `found`, which returns `false` to stop.
fn walk_matching(
    store: &RepoStore,
    query: &Query,
    chunk: usize,
    stop: &dyn Fn() -> bool,
    found: &mut dyn FnMut(&[&Commit]) -> bool,
) -> Result<Scanned, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let table = store.table().map_err(|error| error.to_string())?;
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, table).map_err(|error| error.to_string())?;
    let mut author = Vec::new();
    loop {
        if stop() {
            return Ok(Scanned::Stopped);
        }
        let read = walk
            .next_chunk(chunk, stop)
            .map_err(|error| error.to_string())?;
        let hits: Vec<&Commit> = read
            .commits
            .iter()
            .filter(|commit| {
                // Name and email as `git log` shows them, reused per commit.
                author.clear();
                author.extend_from_slice(&commit.author.name);
                author.extend_from_slice(b" <");
                author.extend_from_slice(&commit.author.email);
                author.push(b'>');
                query.matches(commit.id, commit.subject(), &author, &commit.message)
            })
            .collect();
        if !found(&hits) {
            return Ok(Scanned::Stopped);
        }
        if read.stopped {
            return Ok(Scanned::Stopped);
        }
        if read.done {
            return Ok(Scanned::Finished);
        }
    }
}

/// The search on screen: its query, the hits found so far in walk order, which
/// one the selection is on, and where the selection was before it began.
///
/// Hits are kept as the batches the job sent, shared, because `AppState` is
/// cloned on every event: a broad query on a big repository would otherwise
/// copy every hit per key press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Search {
    /// What is being searched for.
    pub query: Query,
    /// The job searching, once it has started and while it runs.
    pub job: Option<JobId>,
    /// Searches asked for whose job has not said it started yet. Only the
    /// last to start is this search's; earlier ones are superseded.
    pub unstarted: usize,
    /// Hits found, in batches, in walk order.
    pub batches: Vec<Arc<[Oid]>>,
    /// The hit the selection was sent to, if any.
    pub current: Option<usize>,
    /// Where the search stands.
    pub state: SearchState,
    /// Cursor row and viewport top before the search moved them.
    pub prior: (usize, usize),
    /// The commit a jump is still seeking, while it walks.
    pub seeking: Option<Oid>,
    /// The last hit sought is not a row in the view on screen: a view he
    /// chose hides it (first-parent, a hidden stash). A hit the default
    /// fold hides is revealed instead (ease-pass 06c).
    pub missed: bool,
    /// Of the hits, how many the default fold hides, as the walk last
    /// counted them (ease-pass 06c). Every hit is still in the count; the
    /// line says how many are in folded merges while this is above zero.
    pub folded: usize,
    /// Of the hits, how many the walk had not reached when it counted:
    /// asked once more when the walk completes.
    pub unwalked: usize,
    /// The number of the fold count this search asked the walk for last
    /// (`History::ask_folded`); an answer with any other number is not its.
    pub folded_ask: Option<u64>,
}

impl Search {
    /// A search for `query` just asked for, from a selection at `prior`.
    pub fn new(query: Query, unstarted: usize, prior: (usize, usize)) -> Self {
        Search {
            query,
            job: None,
            unstarted,
            batches: Vec::new(),
            current: None,
            state: SearchState::Running { hits: 0 },
            prior,
            seeking: None,
            missed: false,
            folded: 0,
            unwalked: 0,
            folded_ask: None,
        }
    }

    /// How many hits so far.
    pub fn hits(&self) -> usize {
        self.batches.iter().map(|batch| batch.len()).sum()
    }

    /// The `index`th hit, in walk order.
    pub fn hit(&self, index: usize) -> Option<Oid> {
        let mut index = index;
        for batch in &self.batches {
            match batch.get(index) {
                Some(id) => return Some(*id),
                None => index -= batch.len(),
            }
        }
        None
    }

    /// The hit `step` away from the current one (`n` is `+1`, `N` is `-1`),
    /// wrapping at either end; the first hit when none is current yet.
    pub fn step(&mut self, forward: bool) -> Option<Oid> {
        let hits = self.hits();
        if hits == 0 {
            return None;
        }
        let next = match (self.current, forward) {
            (None, _) => 0,
            (Some(at), true) => (at + 1) % hits,
            (Some(at), false) => (at + hits - 1) % hits,
        };
        self.current = Some(next);
        self.hit(next)
    }

    /// The footer's words: query, state, how many hits are in folded merges
    /// (only while some are), position, and the keys that apply, spelled
    /// from `keys` ([`keys_words`]).
    pub fn line(&self, keys: &mergewright_ui::keymap::Keymap) -> String {
        let position = match self.current {
            Some(at) => format!(" ({}/{})", at + 1, self.hits()),
            None => String::new(),
        };
        let missed = if self.missed {
            "  (not in this view)"
        } else {
            ""
        };
        // Shown only while there is something to say (status rule).
        let folded = if self.folded > 0 {
            format!(" \u{b7} {} in folded merges", self.folded)
        } else {
            String::new()
        };
        let running = matches!(self.state, SearchState::Running { .. });
        let keys = keys_words(running, self.hits(), keys);
        format!(
            "/{}  {}{folded}{position}{missed}{keys}",
            self.query.text(),
            self.state.describe()
        )
    }
}

/// The search line's keys: `  n/N next/prev  esc close` (`esc cancel`
/// while running; no `n/N` with no hits). Spelled from the graph's rows —
/// `n`/`N` step on the translated key (`update/search.rs` `search_step`),
/// `esc` is the graph's `Esc` ladder. An unbound key's clause goes; with
/// one of `n`/`N` unbound the other keeps its own verb (`n next`).
#[must_use]
pub fn keys_words(running: bool, hits: usize, keys: &mergewright_ui::keymap::Keymap) -> String {
    use mergewright_ui::keymap::ActionId;
    let mut parts = Vec::new();
    if hits > 0 {
        let next = keys.spell_keys(&[ActionId("app.next_match")]);
        let prev = keys.spell_keys(&[ActionId("app.prev_match")]);
        match (next, prev) {
            (Some(next), Some(prev)) => parts.push(format!("{next}/{prev} next/prev")),
            (Some(next), None) => parts.push(format!("{next} next")),
            (None, Some(prev)) => parts.push(format!("{prev} prev")),
            (None, None) => {}
        }
    }
    let verb = if running { "cancel" } else { "close" };
    parts.extend(keys.key_verb(&[ActionId("app.back")], verb));
    let mut line = String::new();
    for part in parts {
        line.push_str("  ");
        line.push_str(&part);
    }
    line
}

#[cfg(test)]
mod tests;
