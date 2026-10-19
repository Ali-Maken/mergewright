//! The gestures a scenario makes that are more than one key: moving the
//! cursor to a row, picking a verb from a menu, dragging a badge, staging a
//! file through the changes pane.

use mergewright::update::AppState;
use mergewright::update::detail::Loading;
use mergewright_core::event::{KeyCode, MouseButton, MouseKind};
use mergewright_core::job::JobId;
use mergewright_ui::detail::DetailFocus;

use super::run::Walk;
use crate::repo_truth::check;

impl Walk<'_> {
    /// `g`, then `j` until `here` holds of the selected row.
    pub(super) fn cursor_to(
        &mut self,
        state: &AppState,
        here: impl Fn(&AppState) -> bool,
    ) -> Result<AppState, String> {
        let mut state = self.app.key(state, KeyCode::Char('g'));
        for _ in 0..40 {
            if here(&state) {
                return Ok(state);
            }
            state = self.app.key(&state, KeyCode::Char('j'));
        }
        Err(format!(
            "forty rows down and never found it: {:?}",
            subjects(&state)
        ))
    }

    /// `.` (unless the menu is already open), `j` to the verb whose label
    /// starts with `prefix`, `Enter`.
    pub(super) fn menu_pick(
        &mut self,
        state: AppState,
        prefix: &str,
        open: bool,
    ) -> Result<AppState, String> {
        let mut state = if open {
            self.app.key(&state, KeyCode::Char('.'))
        } else {
            state
        };
        let labels: Vec<String> = state
            .context_menu
            .as_ref()
            .ok_or_else(|| format!("`.` opened no menu; status {:?}", state.status))?
            .menu
            .items
            .iter()
            .map(|item| item.label.to_string())
            .collect();
        for _ in 0..labels.len() {
            let menu = &state
                .context_menu
                .as_ref()
                .ok_or_else(|| "the menu closed while moving in it".to_owned())?
                .menu;
            if menu.items[menu.cursor].label.starts_with(prefix) {
                return Ok(self.app.key(&state, KeyCode::Enter));
            }
            state = self.app.key(&state, KeyCode::Char('j'));
        }
        Err(format!("no menu row starts with {prefix:?}: {labels:?}"))
    }

    /// Pick the first draggable badge up with the mouse, drag it onto the
    /// row carrying `badge`, release: three real mouse events.
    pub(super) fn drag_badge_onto(
        &mut self,
        state: &AppState,
        badge: &str,
    ) -> Result<AppState, String> {
        let (state, column, _from) = self.grab_a_badge(state)?;
        let from_row = state
            .drag
            .as_ref()
            .map(|drag| drag.from_row)
            .ok_or_else(|| "no badge is in the air".to_owned())?;
        let onto = row_of_badge(&state, badge)
            .ok_or_else(|| format!("no row carries {badge}: {:?}", subjects(&state)))?;
        check(onto != from_row, || {
            format!("{badge} is on the row the badge came from")
        })?;
        // Where the target is drawn is the app's to say: the cursor row's
        // second line moves every row below it down one (ease-pass/07b).
        let onto_screen_row = mergewright::update::history_row_line(&state, onto)
            .ok_or_else(|| format!("{badge}'s row is above the screen"))?;
        let state = self.app.mouse(
            &state,
            MouseKind::Drag(MouseButton::Left),
            column,
            onto_screen_row,
        );
        check(
            state.drag.as_ref().and_then(|drag| drag.over) == Some(onto),
            || format!("the badge is not over {badge}'s row"),
        )?;
        let state = self.app.mouse(
            &state,
            MouseKind::Up(MouseButton::Left),
            column,
            onto_screen_row,
        );
        check(state.context_menu.is_some(), || {
            format!("the release opened no drop menu: {:?}", state.status)
        })?;
        Ok(state)
    }

    fn grab_a_badge(&mut self, state: &AppState) -> Result<(AppState, u16, u16), String> {
        for screen_row in 1..8_u16 {
            for column in 0..self.columns {
                let pressed = self.app.mouse(
                    state,
                    MouseKind::Down(MouseButton::Left),
                    column,
                    screen_row,
                );
                if pressed.drag.is_some() {
                    return Ok((pressed, column, screen_row));
                }
            }
        }
        Err(format!(
            "no draggable badge in the top rows of a {}-column frame",
            self.columns
        ))
    }

    /// The changes pane from the top row, `Space` on `name`, the write,
    /// the list re-read, `Esc` back to the graph.
    pub(super) fn stage(&mut self, state: &AppState, name: &str) -> Result<AppState, String> {
        let state = self.app.key(state, KeyCode::Char('g'));
        let state = if state.on_working_tree {
            state
        } else {
            self.app.key(&state, KeyCode::Char('k'))
        };
        check(state.on_working_tree, || {
            "`k` from the top row did not select the uncommitted-changes row".to_owned()
        })?;
        let state = self.app.key(&state, KeyCode::Enter);
        let state = self.pump(state, "the changes pane's file list", |state| {
            matches!(state.detail.files, Loading::Ready(_))
        })?;
        let state = crate::app_harness::cursor_to(self.app, state, name);
        let state = self.app.key(&state, KeyCode::Char(' '));
        let state = self.settle_write(state, &format!("staging {name}"))?;
        let state = self.pump(state, "the file list after the stage", |state| {
            matches!(state.detail.files, Loading::Ready(_)) && state.journal.reading.is_none()
        })?;
        Ok(self.app.key(&state, KeyCode::Escape))
    }
}

/// The selected row carries the branch or tag badge `name`.
pub(super) fn on_badge(state: &AppState, name: &str) -> bool {
    let Some(refs) = state.refs.as_deref() else {
        return false;
    };
    let Some(row) = state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
    else {
        return false;
    };
    refs.on(row.commit.id).iter().any(|label| {
        matches!(label, mergewright::refs::RefLabel::Branch { name: found, .. } if found == name.as_bytes())
            || matches!(label, mergewright::refs::RefLabel::Tag(found) if found == name.as_bytes())
    })
}

/// The selected row's subject is `subject`.
pub(super) fn on_subject(state: &AppState, subject: &str) -> bool {
    state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
        .is_some_and(|row| row.commit.subject() == subject.as_bytes())
}

/// Which drawn row carries the branch badge `name`.
fn row_of_badge(state: &AppState, name: &str) -> Option<usize> {
    let refs = state.refs.as_deref()?;
    let window = state.history.as_ref()?.window.as_ref()?;
    window.rows.iter().position(|row| {
        refs.on(row.commit.id).iter().any(|label| {
            matches!(label, mergewright::refs::RefLabel::Branch { name: found, .. } if found == name.as_bytes())
        })
    })
}

/// The drawn rows' subjects, for a failure message.
fn subjects(state: &AppState) -> Vec<String> {
    state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .map(|window| {
            window
                .rows
                .iter()
                .map(|row| String::from_utf8_lossy(row.commit.subject()).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

// ───────────────────────────── the resolver (m6-conflicts/07) ─────────────────────────────

impl Walk<'_> {
    /// [`super::steps::Step::Resolve`]: the resolver open and idle on
    /// `path`, reached the way a person reaches it — off the banner, into
    /// the changes pane, `j`/`k` to the path's row, `⏎`.
    pub(super) fn resolve(&mut self, state: AppState, path: &str) -> Result<AppState, String> {
        let mut state = state;
        if state
            .resolver
            .as_ref()
            .is_some_and(|open| open.path.as_os_str() != path)
        {
            state = self.app.key(&state, KeyCode::Escape);
        }
        if state
            .resolver
            .as_ref()
            .is_none_or(|open| open.path.as_os_str() != path)
        {
            // `Esc`, not `Tab`: since m6-conflicts/12 a launch into a live
            // operation lands with the changes pane open and focused, and
            // `Esc` hands the keyboard back to it where `Tab` would move on
            // to the graph. (A first `Esc` may only dismiss a sticky line.)
            let mut guard = 0;
            while state.banner_focus {
                check(guard < 6, || "Esc never let the banner go".to_owned())?;
                state = self.app.key(&state, KeyCode::Escape);
                guard += 1;
            }
            if state.on_changes_pane() && state.detail.focus == DetailFocus::Graph {
                // The pane is open on the work but the graph has the keys:
                // one `Tab` takes them to its file list.
                state = self.app.key(&state, KeyCode::Tab);
            }
            if !state.on_changes_pane() {
                state = self.app.key(&state, KeyCode::Char('g'));
                if !state.on_working_tree {
                    state = self.app.key(&state, KeyCode::Char('k'));
                }
                check(state.on_working_tree, || {
                    "`k` from the top row did not select the uncommitted-changes row".to_owned()
                })?;
                state = self.app.key(&state, KeyCode::Enter);
            }
            state = self.pump(state, "the changes pane's file list", |state| {
                matches!(state.detail.files, Loading::Ready(_))
                    && state.on_changes_pane()
                    && state.operating.is_none()
            })?;
            state = crate::app_harness::cursor_to(self.app, state, path);
            state = self.app.key(&state, KeyCode::Enter);
        }
        let state = self.pump(state, "the resolver's read", |state| {
            state.operating.is_none()
                && state.resolver.as_ref().is_some_and(|open| {
                    open.job.is_none()
                        && open.base_job.is_none()
                        && (open.file.is_some() || open.whole_file.is_some())
                })
        })?;
        let on = state
            .resolver
            .as_ref()
            .map(|open| open.path.display().to_string());
        check(on.as_deref() == Some(path), || {
            format!(
                "the resolver opened on {on:?}, not {path}; status {:?}",
                state.status
            )
        })?;
        Ok(state)
    }

    /// [`super::steps::Step::HandOver`]: press `key`, carry out the
    /// terminal hand-over its commands ask for with the loop's own command
    /// builders, and feed the answer back — every other command goes to the
    /// pool as `App::press` would send it.
    pub(super) fn hand_over(
        &mut self,
        state: &AppState,
        what: &str,
        key: mergewright_core::event::KeyPress,
    ) -> Result<AppState, String> {
        use mergewright::event_loop::outside::{editor_command, mergetool_command};
        use mergewright::jobs::Dispatch as _;
        use mergewright::update::{AppEvent, Command, Product, update};

        let (next, commands) = crate::app_harness::press_for_commands(state, key);
        let mut state = next;
        let repo = self.fixture.repo().to_path_buf();
        let mut handed = 0;
        for command in commands {
            let event = match command {
                Command::OpenExternalEditor { path, line } => {
                    let editor = fixture_editor(&repo, what)?;
                    let status = editor_command(&editor, &path, line, Some(&repo)).status();
                    // The words `Handover::open` uses (event_loop/outside.rs).
                    let result = match status {
                        Ok(status) if status.success() => Ok(()),
                        Ok(status) => Err(match status.code() {
                            Some(code) => format!("{editor} exited {code}, message unchanged"),
                            None => format!("{editor} was killed, message unchanged"),
                        }),
                        Err(why) => Err(format!("could not run {editor}: {why}")),
                    };
                    AppEvent::Produced(JobId::new(0), Product::HandedOver(result))
                }
                Command::OpenMergetool { path } => {
                    let status = mergetool_command(&path, &repo).status();
                    // The words `Handover::mergetool` uses (event_loop/outside.rs).
                    let result = match status {
                        Ok(status) if status.success() => Ok(()),
                        Ok(status) => Err(match status.code() {
                            Some(code) => {
                                format!("mergetool exited {code} for {}", path.display())
                            }
                            None => format!("mergetool was killed for {}", path.display()),
                        }),
                        Err(why) => Err(format!("could not run git mergetool: {why}")),
                    };
                    AppEvent::Produced(JobId::new(0), Product::Mergetool { path, result })
                }
                other => {
                    self.app.jobs.dispatch(other);
                    continue;
                }
            };
            handed += 1;
            self.seen.events.push(format!("{event:?}"));
            let (after, more) = update(&state, event);
            state = after;
            for command in more {
                self.app.jobs.dispatch(command);
            }
        }
        check(handed == 1, || {
            format!(
                "{what}: the key handed the terminal over {handed} times; status {:?}",
                state.status
            )
        })?;
        Ok(state)
    }
}

/// The editor the fixture set, as the app's hand-over would find it:
/// `Handover::git_editor` asks git in the git directory
/// (`mergewright_git::editor::git_editor`, `git var GIT_EDITOR`). The same
/// question here, with the environment's editor variables taken out: git
/// answers them before `core.editor`, so a developer's `GIT_EDITOR=true`
/// would otherwise decide what the scenario's fixture set.
fn fixture_editor(repo: &std::path::Path, what: &str) -> Result<String, String> {
    let asked = std::process::Command::new("git")
        .args(["var", "GIT_EDITOR"])
        .current_dir(repo.join(".git"))
        .env_remove("GIT_EDITOR")
        .env_remove("VISUAL")
        .env_remove("EDITOR")
        .output()
        .map_err(|why| format!("{what}: git var GIT_EDITOR: {why}"))?;
    check(asked.status.success(), || {
        format!(
            "{what}: git names no editor: {}",
            String::from_utf8_lossy(&asked.stderr)
        )
    })?;
    Ok(String::from_utf8_lossy(&asked.stdout).trim().to_owned())
}
