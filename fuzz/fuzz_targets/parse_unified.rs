//! Fuzzes `graft_git::diff::parse_unified` with arbitrary bytes: it must
//! return `Ok` or `Err`, never panic, for any input at all.
#![no_main]

use graft_git::diff::parse_unified;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parse_unified(data);
});
