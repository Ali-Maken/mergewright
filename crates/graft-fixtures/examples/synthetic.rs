//! Builds (or finds) one synthetic repository of any size and prints where it
//! is — for the memory curve (ticket m1-the-graph/46), which measures sizes no
//! test builds:
//!
//! ```sh
//! cargo run --release -p graft-fixtures --features heavy-tests --example synthetic -- 1000000 [--commit-graph] [--realistic]
//! ```
//!
//! Heavy tier only: the example does not compile without `heavy-tests`.
//! Prints one line, `<path>\t<MiB on disk>\t<seconds>\t<built|cached>`.

use std::process::ExitCode;

use graft_fixtures::synthetic::{self, Scale};

fn main() -> ExitCode {
    let mut commits = None;
    let mut commit_graph = false;
    let mut realistic = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--commit-graph" => commit_graph = true,
            "--realistic" => realistic = true,
            number => match number.parse::<u32>() {
                Ok(n) => commits = Some(n),
                Err(why) => {
                    eprintln!("synthetic: {number:?} is not a commit count: {why}");
                    return ExitCode::FAILURE;
                }
            },
        }
    }
    let Some(commits) = commits else {
        eprintln!("usage: synthetic <commits> [--commit-graph] [--realistic]");
        return ExitCode::FAILURE;
    };
    let mut scale = Scale::of(commits);
    if commit_graph {
        scale = scale.with_commit_graph();
    }
    if realistic {
        scale = scale.with_realistic_commits();
    }
    match synthetic::ensure(scale) {
        Ok(report) => {
            // Display only: a fixture's size in MiB to one decimal place.
            #[allow(clippy::cast_precision_loss)]
            let mib = report.bytes as f64 / (1024.0 * 1024.0);
            println!(
                "{}\t{mib:.1}\t{:.1}\t{}",
                report.root.display(),
                report.elapsed.as_secs_f64(),
                if report.built { "built" } else { "cached" }
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("synthetic: {why}");
            ExitCode::FAILURE
        }
    }
}
