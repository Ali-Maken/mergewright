//! `Ctrl+Z`: stop this process as any terminal program is stopped.

use std::io;
use std::process::Command;

/// Stop Mergewright the way `Ctrl+Z` stops any other terminal program, by sending
/// this process `SIGTSTP`.
///
/// Through `kill(1)` rather than `libc::raise`: neither `libc` nor `nix` is a
/// dependency of this crate, and adding one for a single signal — on a path
/// the user has to turn on in config — costs more than the spawn does
/// (the spec says so too).
///
/// The shell puts Mergewright back in the foreground with `fg`, and the call
/// returns then; the terminal has been restored around it by
/// [`run_outside_terminal`](super::run_outside_terminal), so `fg` lands the user back in a redrawn Mergewright.
///
/// # Errors
/// `kill` could not be run, or refused.
pub fn stop_this_process() -> io::Result<()> {
    let status = Command::new("kill")
        .args(["-TSTP", &std::process::id().to_string()])
        .status()?;
    if status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!("kill -TSTP: {status}")))
}
