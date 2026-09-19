//! Real remotes for tests: something to fetch from, pull from and push to.
//!
//! M4's promise is that you never leave Graft to fetch, pull or push — not
//! for a password, a rejection or a hang. Each clause needs a remote that
//! produces that condition on demand, so there are three:
//!
//! - [`LocalRemote`] — a bare repository and a working clone of it, joined
//!   by a real `origin` with real upstreams, over the `file` transport. For
//!   ahead, behind, diverged, rejected-push and pull tests.
//! - [`HttpRemote`] — the same bare repository served over git's smart HTTP
//!   protocol (`git http-backend`) with Basic authentication, on a loopback
//!   port. For the password prompt: a wrong password is a `401`, the right
//!   one fetches and pushes.
//! - [`HungRemote`] — a loopback port that accepts a connection and never
//!   answers. For the hang: a fetch that has to be cancelled.
//!
//! **Not covered:** TLS (plain HTTP is enough to exercise Basic auth and the
//! askpass path; TLS would add certificates and nothing Graft reacts to) and
//! SSH (it would need an `sshd` and host keys — out of M4's test scope).
//!
//! Everything here is laptop-safe (PROMPT.md §7.1): loopback only, ephemeral
//! ports only, every file in one temporary directory deleted on drop, and
//! every `git` child hermetic — no system or global config, `HOME` pointed
//! into the scratch directory so the user's credential helpers, `push.default`
//! and `pull.rebase` can never reach a test, `GIT_TERMINAL_PROMPT=0`, a fixed
//! identity and clock, and a deadline.

mod http;
mod hung;
mod local;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::process::Command;

pub use http::HttpRemote;
pub use hung::HungRemote;
pub use local::LocalRemote;

use crate::git::{self, GitError, Input};

/// Runs `git <args>` in `cwd` with the corpus's fixed identity and clock, no
/// machine config, `HOME` (and `XDG_CONFIG_HOME`) set to `home`, and `env`
/// added on top — the one runner every remote fixture shares.
pub(crate) fn run_git(
    home: &Path,
    cwd: &Path,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<Vec<u8>, GitError> {
    git::run_built(
        |cwd, args| {
            let mut command = hermetic_command(home, cwd, args);
            command.envs(env.iter().copied());
            command
        },
        cwd,
        args,
        &Input::Nothing,
    )
}

/// The unspawned command [`run_git`] would run, for a caller that needs the
/// child itself (a fetch it will kill).
pub(crate) fn hermetic_command(home: &Path, cwd: &Path, args: &[&str]) -> Command {
    let mut command = git::hermetic(cwd, args);
    command.env("HOME", home).env("XDG_CONFIG_HOME", home);
    command
}
