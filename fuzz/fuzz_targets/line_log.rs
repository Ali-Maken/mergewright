//! Fuzzes `mergewright_git::trace::parse_line_log` with arbitrary bytes: it must
//! return `Ok` or `Err`, never panic (m7-power-tools/18).
#![no_main]

use mergewright_git::trace::parse_line_log;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Any answer is fine; a panic is the failure.
    drop(parse_line_log(data));
});
