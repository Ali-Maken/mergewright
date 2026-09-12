//! Peak resident memory, the way PROMPT.md §7.1 says to measure it.
//!
//! The heavy tier has one memory gate: loading and scrolling the 100k-commit
//! repository must stay under 300 MB, "measured with `getrusage`". This
//! module is that measurement and nothing else — the gate itself is a later
//! ticket's assertion.
//!
//! # The one thing to know before asserting on this
//!
//! `ru_maxrss` is a **high-water mark for the whole process**, and it never
//! falls. Two consequences, both of which have bitten real suites:
//!
//! - [`Measured::rise`] under-reports whenever something earlier in the same
//!   test binary peaked higher. If an earlier test touched 400 MB, work that
//!   touches 300 MB shows a rise of zero.
//! - It counts every thread and every allocation in the process, not just the
//!   work in the closure.
//!
//! So a budget test asserts on [`Measured::peak`] — an absolute number, with
//! the whole process's footprint in it — and runs where nothing else large
//! has run first: its own integration-test binary, at `--test-threads=1` or
//! alone in a heavy-tier run. `rise` is for reporting and for a test that
//! only needs "this work allocated roughly that much", never for a gate.
//!
//! macOS and Linux only (ADR-0005), which is also the two platforms whose
//! `ru_maxrss` units differ: bytes on Darwin, kibibytes on Linux.

use std::io;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
compile_error!(
    "graft-fixtures::memory reads ru_maxrss, whose unit is platform-specific. \
     Graft targets macOS and Linux only (ADR-0005); add the unit for a new \
     platform deliberately rather than guessing it."
);

/// What `ru_maxrss` counts in, on this platform.
///
/// Darwin reports bytes; Linux reports kibibytes. Getting this wrong is a
/// factor of 1024 on a 300 MB gate, so it is stated per platform rather than
/// inferred.
#[cfg(target_os = "macos")]
const MAXRSS_BYTES: u64 = 1;
#[cfg(target_os = "linux")]
const MAXRSS_BYTES: u64 = 1024;

/// Why a peak-memory reading could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// `getrusage` itself failed.
    #[error("getrusage(RUSAGE_SELF) failed: {0}")]
    Getrusage(#[source] io::Error),
    /// The kernel reported a negative maximum resident set size, which is not
    /// a size. Never seen; reported rather than clamped, because a clamp here
    /// would turn a broken reading into a passing budget test.
    #[error("getrusage reported a negative ru_maxrss ({0}), which is not a size")]
    Negative(i64),
}

/// One measurement: where the process's peak stood before the work, and where
/// it stood after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measured {
    /// The process's peak resident set before the work, in bytes.
    pub before: u64,
    /// The process's peak resident set after the work, in bytes. This is the
    /// number a budget asserts on.
    pub peak: u64,
}

impl Measured {
    /// How much higher the peak is than it was, in bytes.
    ///
    /// Zero whenever the work stayed under a peak something else had already
    /// set — see this module's header. Never use it as a gate.
    pub fn rise(self) -> u64 {
        self.peak.saturating_sub(self.before)
    }
}

/// The largest resident set this process has had so far, in bytes.
pub fn peak_rss() -> Result<u64, MemoryError> {
    let max_rss = max_rss()?;
    u64::try_from(max_rss)
        .map(|kept| kept * MAXRSS_BYTES)
        .map_err(|_| MemoryError::Negative(max_rss))
}

/// Runs `work` and reports the process's peak resident set either side of it.
///
/// The work's own value is handed back untouched, so a measurement can wrap
/// an existing call without restructuring it.
pub fn measure<T>(work: impl FnOnce() -> T) -> Result<(T, Measured), MemoryError> {
    let before = peak_rss()?;
    let produced = work();
    let peak = peak_rss()?;
    Ok((produced, Measured { before, peak }))
}

/// `ru_maxrss`, in whatever unit this platform reports it in.
#[allow(
    unsafe_code,
    reason = "getrusage has no safe wrapper in the standard library, and \
              PROMPT.md §7.1 names it as the way the memory gate is measured; \
              ADR-0015 covers this call and its soundness argument"
)]
fn max_rss() -> Result<i64, MemoryError> {
    // Zeroed rather than left uninitialised: every field of `rusage` is an
    // integer or a `timeval` of two integers, so all-zeroes is a valid value
    // and the struct is initialised whether or not the kernel writes every
    // field (ADR-0015).
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: `getrusage` writes one `rusage` through the pointer, which
    // points at a live, correctly aligned allocation of exactly that type,
    // owned by this frame and not aliased. `RUSAGE_SELF` is a valid `who`.
    let queried = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if queried != 0 {
        return Err(MemoryError::Getrusage(io::Error::last_os_error()));
    }
    // SAFETY: zero-initialised above, so it is initialised regardless of what
    // the call wrote.
    let usage = unsafe { usage.assume_init() };
    Ok(usage.ru_maxrss)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;

    /// How much more than the current high-water mark the known allocation
    /// asks for. Enough to be far outside any page-accounting noise, small
    /// enough that the test costs a fraction of the fast tier's 2 GB budget.
    const SLAB: u64 = 64 * MIB;

    /// The reading is a real one: a known allocation moves it by a known
    /// amount.
    ///
    /// The allocation is `peak + SLAB`, not `SLAB`, and that is the whole
    /// trick. `ru_maxrss` is a high-water mark, so an allocation smaller than
    /// the mark already set moves nothing. A buffer bigger than the mark is
    /// resident on its own, so the new mark is at least its size, and the
    /// rise is therefore at least `SLAB` whatever the process did before.
    #[test]
    fn a_known_allocation_raises_the_peak_by_at_least_its_size() {
        let before = peak_rss().unwrap();
        assert!(
            before < 512 * MIB,
            "this test allocates `peak + {SLAB}` bytes and the peak is already \
             {before} bytes; something before it used far more memory than the \
             fast tier's budget allows"
        );

        let want = usize::try_from(before + SLAB).unwrap();
        let (touched, measured) = measure(|| {
            let mut slab = vec![0u8; want];
            // `vec![0u8; n]` is a `calloc`, which on both platforms hands back
            // pages that are not resident until something writes to them. One
            // byte per 4 KiB faults in every page on Linux (4 KiB pages) and
            // on macOS (16 KiB pages) alike.
            for page in slab.chunks_mut(4096) {
                page[0] = 1;
            }
            std::hint::black_box(&slab).len()
        })
        .unwrap();

        assert_eq!(touched, want, "the whole buffer was written");
        assert!(
            measured.peak >= measured.before,
            "a high-water mark cannot fall: {measured:?}"
        );
        // Three quarters of the slab, not all of it: the guarantee above is
        // exact, and the slack is only there so page accounting or a rounding
        // difference between platforms cannot make a correct reading fail.
        let least = SLAB / 4 * 3;
        assert!(
            measured.rise() >= least,
            "expected the peak to rise by at least {least} bytes after \
             touching {want} of them, but {measured:?} rose by {}",
            measured.rise()
        );
    }

    /// The plain reading agrees with the measured one and is a plausible size
    /// for a running process — not zero, and not a kibibyte count mistaken
    /// for bytes.
    #[test]
    fn peak_rss_reports_a_plausible_size() {
        let peak = peak_rss().unwrap();
        assert!(
            peak > MIB,
            "a running Rust test binary holds more than a mebibyte, so {peak} \
             bytes means the ru_maxrss unit for this platform is wrong"
        );
        assert!(
            peak_rss().unwrap() >= peak,
            "a high-water mark cannot fall between two readings"
        );
    }

    /// A measurement hands the work's own value straight back.
    #[test]
    fn measure_returns_what_the_work_returned() {
        let (answer, _) = measure(|| "the work's own value").unwrap();
        assert_eq!(answer, "the work's own value");
    }
}
