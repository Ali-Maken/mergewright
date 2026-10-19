//! `cargo run -p mergewright-fixtures --example team_path`
//!
//! Builds the MONOREPO team fixture ([`TeamSpec::MONOREPO`]) into the
//! fixture cache if it isn't already built, then prints its absolute path.
//! Exists so a perf script (`scripts/ease-pixel-lag.sh`) can get the
//! fixture without running a heavy test for its side effect.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use mergewright_fixtures::TeamSpec;
use mergewright_fixtures::team;

fn main() -> ExitCode {
    match team::borrow(&TeamSpec::MONOREPO) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("error: {why}");
            ExitCode::FAILURE
        }
    }
}
