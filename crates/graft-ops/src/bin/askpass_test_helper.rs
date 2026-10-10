//! **Test-only.** The askpass helper half, standing in for Graft's own binary
//! in askpass mode (M4 group 05b) so `graft-ops`'s integration tests can
//! prove git → helper → `AskPass` → git end to end today. It does exactly
//! what the app's mode will: read `TRIB_ASKPASS_SOCKET`, ask over it with
//! git's prompt (`argv[1]`), print the answer. Nothing ships it.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use graft_git::run::askpass::respond;

fn main() -> ExitCode {
    let Some(socket) = std::env::var_os(graft_core::on_disk::ASKPASS_SOCKET_VAR) else {
        eprintln!("askpass_test_helper: TRIB_ASKPASS_SOCKET is not set");
        return ExitCode::FAILURE;
    };
    // git always passes the prompt; with none, asking with an empty prompt
    // is still a valid question — the handler decides, nothing is hidden.
    let prompt = std::env::args().nth(1).unwrap_or_default();
    let mut out = std::io::stdout().lock();
    match respond(&PathBuf::from(socket), &prompt, &mut out).and_then(|()| {
        out.flush()
            .map_err(graft_git::run::askpass::RespondError::Io)
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("askpass_test_helper: {why}");
            ExitCode::FAILURE
        }
    }
}
