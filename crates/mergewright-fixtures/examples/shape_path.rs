//! `cargo run -p mergewright-fixtures --example shape_path -- <shape-name>`
//!
//! Builds the corpus if it isn't already built, then prints the absolute
//! path to one named shape's fixture repository — the name is [`Shape::name`]
//! (`criss-cross-merge`, `foxtrot-merge`, `octopus-merge`, ...). Exists so a
//! shell script (`scripts/m1-checkpoint.sh`) can hand a real repository path
//! to `cargo run -- <path>` without embedding Rust.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use mergewright_fixtures::Shape;

fn main() -> ExitCode {
    let Some(name) = std::env::args().nth(1) else {
        eprintln!("usage: shape_path <shape-name>");
        return ExitCode::FAILURE;
    };
    let Some(shape) = Shape::from_name(&name) else {
        eprintln!("error: no such fixture shape: {name}");
        return ExitCode::FAILURE;
    };
    match mergewright_fixtures::try_borrow(shape) {
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
