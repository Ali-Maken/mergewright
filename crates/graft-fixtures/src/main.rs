//! `cargo run -p graft-fixtures` — build the fixture corpus, or confirm it is
//! already built.
//!
//! Prints where the corpus is, whether this run built it, how long it took,
//! how much disk it uses, and any shape this machine cannot hold. Running it
//! twice is the check that the second run is a no-op.

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match graft_fixtures::ensure() {
        Ok(report) => {
            let action = if report.built {
                "built"
            } else {
                "already built"
            };
            println!("corpus   {}", report.root.display());
            println!("state    {action} in {:.2?}", report.elapsed);
            println!("size     {:.1} MiB", bytes_to_mib(report.bytes));
            if report.unavailable.is_empty() {
                println!("skipped  none");
            } else {
                for skipped in &report.unavailable {
                    println!("skipped  {}: {}", skipped.shape, skipped.reason);
                }
            }
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("error: {why}");
            ExitCode::FAILURE
        }
    }
}

fn bytes_to_mib(bytes: u64) -> f64 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a corpus size is reported to one decimal place; f64 is exact far past that"
    )]
    let bytes = bytes as f64;
    bytes / (1024.0 * 1024.0)
}
