//! J's find: the pumps and the replay (split out of `journal_find`).

use super::*;

/// The next job event on the shared channel, for `pump_until_from`. The
/// input thread has finished before any wait, so a key here is a harness
/// fault.
pub(super) fn next_event(
    events: &Receiver<Posted>,
    patience: Duration,
) -> Result<AppEvent, String> {
    loop {
        let posted = events
            .recv_timeout(patience)
            .map_err(|why| why.to_string())?;
        match posted.wake {
            Wake::Event(event) => return Ok(event),
            // The last fence answer of a `pumped` loop that has ended
            // (ADR-0054): nothing waits for it, so it is stepped over.
            Wake::Fence => {}
            other => return Err(format!("a wake no job sends: {}", other.label())),
        }
    }
}

/// Opened at 187×50 at the Unicode tier, rows and refs on hand, the
/// journal's launch read answered.
pub(super) fn ready(
    jobs: &impl Dispatch,
    events: &Receiver<Posted>,
    state: &AppState,
) -> Result<AppState, String> {
    let (mut state, commands) = update(
        state,
        AppEvent::Core(Event::Resize {
            columns: COLUMNS,
            rows: ROWS,
        }),
    );
    for command in commands {
        jobs.dispatch(command);
    }
    state.settled_tier = Some(RenderingTier::Unicode);
    budget_run::pump_until_from(
        jobs,
        |patience| next_event(events, patience),
        state,
        |state| {
            state.refs.is_some()
                && state.journal.reading.is_none()
                && state
                    .history
                    .as_ref()
                    .is_some_and(|h| h.window.as_ref().is_some_and(|w| !w.rows.is_empty()))
        },
    )
}

/// One pump's measurements.
pub(super) struct Pumped {
    pub(super) latencies: Vec<Duration>,
    pub(super) trace: Vec<String>,
    /// Every wake handed to the loop, in order: its kind and its event.
    pub(super) replay: Vec<(Kind, AppEvent)>,
}

/// The real loop over `keys`, sent by a [`KeyFeed`] a tick apart into the
/// channel the jobs report into. Ends once the last key's frame is drawn
/// and no rows it asked for are out.
pub(super) fn pumped<D: Dispatch>(
    jobs: &D,
    events: &Receiver<Posted>,
    post: &mpsc::Sender<Posted>,
    session: &mut Session,
    state: AppState,
    keys: &[KeyCode],
    control: Control,
) -> Result<Pumped, String> {
    let script: Vec<(KeyCode, Duration)> = keys.iter().map(|key| (*key, TICK)).collect();
    let sink = Sink::timed(true).answering(post.clone());
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(sink.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, COLUMNS, ROWS)),
        },
    )
    .map_err(|why| format!("in-memory terminal: {why}"))?;
    let watching = Watching::new(jobs, Vec::new());
    let feed = KeyFeed::start(script.clone(), post.clone())
        .map_err(|why| format!("the input thread did not start: {why}"))?;
    let mut wakes = Paced::new(events, script, &watching, sink, feed.log());
    wakes.no_prefetch = control == Control::NoPrefetch;
    let exit = pump(&mut terminal, &mut wakes, &watching, session, state);
    // Before any early return: see `run::scroll`.
    let stamps = feed.finish();
    let exit = exit.map_err(|why| format!("the loop failed: {why}"))?;
    stamps?;
    if !matches!(exit, Exit::Quit) {
        return Err(format!(
            "the loop ended with {exit:?}, not at the end of the script"
        ));
    }
    if let Some(failure) = wakes.failure.take() {
        return Err(failure);
    }
    if wakes.seen.is_some() {
        return Err("the loop ended with a wake taken off the channel and not handed".to_owned());
    }
    wakes.sink.were_fenced()?;
    let latencies = wakes.latencies()?;
    let trace = wakes.trace(keys.len());
    let replay = wakes
        .handed
        .iter()
        .map(|handed| handed.kind)
        .zip(std::mem::take(&mut wakes.replay))
        .collect();
    Ok(Pumped {
        latencies,
        trace,
        replay,
    })
}

/// `state` after `replay`'s events through `update`, up to and including
/// the wake of key `stop` when given. Commands are dropped: the loop
/// already carried them out.
pub(super) fn replayed(
    mut state: AppState,
    replay: &[(Kind, AppEvent)],
    stop: Option<usize>,
) -> AppState {
    for (kind, event) in replay {
        state = update(&state, event.clone()).0;
        if let (Some(at), Kind::Key(key, _)) = (stop, kind)
            && *key == at
        {
            break;
        }
    }
    state
}
