//! The open prompt (`o`) on the empty state: its keys, the typed path
//! resolved (`~`, relative to the shown folder) and `⇥` directory
//! completion. Moved out of `empty_state.rs` (m8/18e B1).

use std::path::{Path, PathBuf};

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_ui::text_input::TextField;

use super::{OutsideState, OutsideStep, Prompt};

pub(super) fn open_prompt_key(
    state: &mut OutsideState,
    mut field: TextField,
    key: KeyPress,
) -> OutsideStep {
    match key.code {
        KeyCode::Escape => return OutsideStep::Stay,
        KeyCode::Enter => {
            if field.text().trim().is_empty() {
                state.prompt = Some(Prompt::Open(field));
                return OutsideStep::Stay;
            }
            let path = resolve(field.text(), &state.folder, state.home.as_deref());
            // The prompt closes. If the open fails, the driver comes back
            // here with the reason as `problem` and the cursor kept.
            return OutsideStep::Open(path);
        }
        KeyCode::Tab => {
            if let Some(completed) = complete(field.text(), &state.folder, state.home.as_deref()) {
                field = TextField::new(&completed);
            }
        }
        KeyCode::Backspace => field.backspace(),
        KeyCode::Delete => field.delete(),
        KeyCode::Left => field.left(),
        KeyCode::Right => field.right(),
        KeyCode::Home => field.home(),
        KeyCode::End => field.end(),
        KeyCode::Char(c) if !key.modifiers.control && !key.modifiers.alt => field.insert(c),
        _ => {}
    }
    state.prompt = Some(Prompt::Open(field));
    OutsideStep::Stay
}

/// A typed path as a real one: `~` is `home`, a relative path is against
/// the shown `folder`.
pub(crate) fn resolve(typed: &str, folder: &Path, home: Option<&Path>) -> PathBuf {
    let typed = typed.trim();
    let expanded = match (typed.strip_prefix('~'), home) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(typed),
    };
    if expanded.is_absolute() {
        expanded
    } else {
        folder.join(expanded)
    }
}

/// `⇥`: complete the last part of `typed` to a directory. One match: the
/// whole name and a `/`. Several: their longest common start. None: `None`.
/// Hidden folders only when the typed part starts with `.`.
pub(crate) fn complete(typed: &str, folder: &Path, home: Option<&Path>) -> Option<String> {
    let (head, part) = match typed.rfind('/') {
        Some(at) => (&typed[..=at], &typed[at + 1..]),
        None => ("", typed),
    };
    let dir = if head.is_empty() {
        folder.to_path_buf()
    } else {
        resolve(head, folder, home)
    };
    // A folder that cannot be listed, an entry that cannot be read, a name
    // that is not UTF-8 (the field holds text): nothing to complete to,
    // which `⇥` shows by leaving the field as it is. Never an error to say:
    // completion is a convenience over typing the name.
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(part) && (part.starts_with('.') || !name.starts_with('.')))
        .collect();
    names.sort();
    let first = names.first()?;
    if names.len() == 1 {
        return Some(format!("{head}{first}/"));
    }
    let common = names.iter().skip(1).fold(first.clone(), |common, name| {
        common
            .chars()
            .zip(name.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a)
            .collect()
    });
    (common.len() > part.len()).then(|| format!("{head}{common}"))
}
