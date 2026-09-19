//! Fuzzes `graft_git::status::parse_porcelain_v2` with arbitrary bytes: it
//! must return `Ok` or `Err`, never panic, for any input at all.
#![no_main]

use graft_git::status::parse_porcelain_v2;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parse_porcelain_v2(data);
});
