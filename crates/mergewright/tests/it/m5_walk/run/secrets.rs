//! The askpass log and the leak check run after every scenario, moved out
//! of `run.rs` to keep that file under its size row.

use super::{Fixture, Path, PathBuf, SECRETS, Seen, check};

/// Where the askpass wrapper writes each invocation's arguments and
/// environment.
fn askpass_log_path(fixture: &Fixture) -> PathBuf {
    fixture.root().join("askpass-invocations.txt")
}

/// What git passed the askpass helper, every time it ran it: one block of
/// arguments and the whole environment per invocation. Empty before the
/// first.
pub(super) fn askpass_log(fixture: &Fixture) -> Result<Vec<u8>, String> {
    let path = askpass_log_path(fixture);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(why) => Err(format!("{}: {why}", path.display())),
    }
}

/// The binary git runs as its askpass helper — `graft` itself under `cargo
/// test`, or this very example — wrapped in a script that records its own
/// arguments and environment before handing over.
///
/// The wrapper is how the walk can say anything about **argv and env from
/// outside the app**: `AppState` cannot show what a child process was
/// started with, and the one child that could carry a credential is this
/// one.
pub(super) fn askpass_wrapper(fixture: &Fixture) -> Result<PathBuf, String> {
    let real = match option_env!("CARGO_BIN_EXE_mwr") {
        Some(graft) => PathBuf::from(graft),
        None => std::env::current_exe().map_err(|why| format!("current_exe: {why}"))?,
    };
    let wrapper = fixture.root().join("askpass-wrapper.sh");
    let log = askpass_log_path(fixture);
    let script = format!(
        "#!/bin/sh\n\
         {{ echo '--- askpass'; for arg in \"$@\"; do echo \"argv: $arg\"; done; env; }} >> '{}'\n\
         exec '{}' \"$@\"\n",
        log.display(),
        real.display()
    );
    std::fs::write(&wrapper, script).map_err(|why| format!("{}: {why}", wrapper.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755))
            .map_err(|why| format!("{}: {why}", wrapper.display()))?;
    }
    Ok(wrapper)
}

/// No password is in the journal on disk, in any event, status line or
/// open-prompt `{state:?}` the loop saw, in what git passed the askpass
/// helper (arguments **and** environment), or in any byte of any file the
/// fixture leaves behind. Run after **every** scenario, not only the one
/// that types one: the rule is about the whole app.
pub fn no_secret_leaked(fixture: &Fixture, seen: &Seen) -> Result<(), String> {
    let askpass = askpass_log(fixture)?;
    for secret in SECRETS {
        let needle = secret.as_bytes();
        check(!contains(&askpass, needle), || {
            // The block itself is not printed: it holds the secret.
            "a password reached the askpass helper's arguments or environment".to_owned()
        })?;
        for line in seen.events.iter().chain(&seen.statuses) {
            check(!line.contains(secret), || {
                "a password reached an event or a status line the loop saw".to_owned()
            })?;
        }
        for dump in &seen.prompts {
            check(!dump.contains(secret), || {
                "a password reached the app state's Debug while the prompt was open".to_owned()
            })?;
        }
        if let Some(path) = file_containing(&fixture.root(), needle)? {
            return Err(format!(
                "a password reached a file the run left behind: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack.len() >= needle.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// The first file under `root` whose bytes hold `needle` — the journal, a
/// git config, a log, a lock file, anything. Names the file rather than its
/// contents.
fn file_containing(root: &Path, needle: &[u8]) -> Result<Option<PathBuf>, String> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(format!("{}: {why}", root.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", root.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|why| format!("{}: {why}", path.display()))?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if let Some(found) = file_containing(&path, needle)? {
                return Ok(Some(found));
            }
        } else if let Ok(bytes) = std::fs::read(&path) {
            // A file that cannot be read holds nothing this walk wrote; a
            // real read failure would already have failed the scenario.
            if contains(&bytes, needle) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}
