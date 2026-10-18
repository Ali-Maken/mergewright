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
    "mergewright-fixtures::memory reads ru_maxrss, whose unit is platform-specific. \
     Mergewright targets macOS and Linux only (ADR-0005); add the unit for a new \
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
    /// `proc_pid_rusage(RUSAGE_INFO_V4)` itself failed.
    #[error("proc_pid_rusage(RUSAGE_INFO_V4) failed: {0}")]
    Footprint(#[source] io::Error),
    /// [`footprint`] was called on a platform other than macOS. Physical
    /// footprint (`ri_phys_footprint`) is a Darwin-specific `libproc`
    /// reading with no Linux equivalent; `ru_maxrss` (see [`peak_rss`])
    /// stays the cross-platform gate (m1-the-graph/57).
    #[error("physical footprint is only readable on macOS")]
    Unsupported,
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

/// This process's physical footprint: anonymous + compressed pages, in
/// bytes — what Activity Monitor's "Memory" column and `/usr/bin/time -l`'s
/// "peak memory footprint" report. Unlike [`peak_rss`], it excludes clean
/// file-backed pages (a mapped pack or commit-graph file), which is why
/// ADR-0026 reports it separately for the no-commit-graph path: `ru_maxrss`
/// there counts pack pages the kernel can reclaim under pressure, footprint
/// does not.
///
/// `current` is this instant's footprint; `peak` is the lifetime maximum
/// (`ri_lifetime_max_phys_footprint`), which — like [`peak_rss`] — never
/// falls within the process's life. The kernel updates `peak`
/// opportunistically rather than on every page fault, so a reading taken
/// right after a fast allocation can (briefly) show `peak` a little below
/// `current`; a caller after the largest rise wants `current.max(peak)`,
/// not `peak` alone (this module's own tests hit this: a ~16 KiB gap under
/// parallel threads).
///
/// macOS only: `Err(`[`MemoryError::Unsupported`]`)` on every other
/// platform (m1-the-graph/57; the no-commit-graph slope's Linux residual is
/// out of this ticket's scope).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footprint {
    /// This instant's physical footprint, in bytes.
    pub current: u64,
    /// The largest physical footprint this process has had so far, in bytes.
    pub peak: u64,
}

/// `proc_pid_rusage`'s `RUSAGE_INFO_V4` flavor (`<sys/proc_info.h>`).
#[cfg(target_os = "macos")]
const RUSAGE_INFO_V4: libc::c_int = 4;

/// The fields of Darwin's `struct rusage_info_v4` (`<sys/resource.h>`) up to
/// and including the two this module reads. `libc` 0.2 does not declare this
/// struct or `proc_pid_rusage`, so both are declared here, scoped to this
/// function, `repr(C)` and field-for-field with the header so the two
/// `u64`s this module reads (`ri_phys_footprint`,
/// `ri_lifetime_max_phys_footprint`) land at the offsets the kernel writes
/// them at. The trailing reserved words keep the struct's total size at
/// least as large as the kernel's write, which is sized by the `RUSAGE_INFO_V4`
/// flavor, not by this declaration.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(
    clippy::struct_field_names,
    reason = "field-for-field with <sys/resource.h>'s struct rusage_info_v4; renaming away the ri_ prefix would make it harder, not easier, to check this against the header"
)]
struct RusageInfoV4 {
    ri_uuid: [u8; 16],
    ri_user_time: u64,
    ri_system_time: u64,
    ri_pkg_idle_wkups: u64,
    ri_interrupt_wkups: u64,
    ri_pageins: u64,
    ri_wired_size: u64,
    ri_resident_size: u64,
    ri_phys_footprint: u64,
    ri_proc_start_abstime: u64,
    ri_proc_exit_abstime: u64,
    ri_child_user_time: u64,
    ri_child_system_time: u64,
    ri_child_pkg_idle_wkups: u64,
    ri_child_interrupt_wkups: u64,
    ri_child_pageins: u64,
    ri_child_elapsed_abstime: u64,
    ri_diskio_bytesread: u64,
    ri_diskio_byteswritten: u64,
    ri_cpu_time_qos_default: u64,
    ri_cpu_time_qos_maintenance: u64,
    ri_cpu_time_qos_background: u64,
    ri_cpu_time_qos_utility: u64,
    ri_cpu_time_qos_legacy: u64,
    ri_cpu_time_qos_user_initiated: u64,
    ri_cpu_time_qos_user_interactive: u64,
    ri_billed_system_time: u64,
    ri_serviced_system_time: u64,
    ri_logical_writes: u64,
    ri_lifetime_max_phys_footprint: u64,
    ri_instructions: u64,
    ri_cycles: u64,
    ri_billed_energy: u64,
    ri_serviced_energy: u64,
    ri_interval_max_phys_footprint: u64,
    ri_runnable_time: u64,
    ri_flags: u64,
    ri_user_ptime: u64,
    ri_system_ptime: u64,
    ri_pinstructions: u64,
    ri_pcycles: u64,
    ri_energy_nj: u64,
    ri_penergy_nj: u64,
    ri_secure_time_in_system: u64,
    ri_secure_ptime_in_system: u64,
    ri_neural_footprint: u64,
    ri_lifetime_max_neural_footprint: u64,
    ri_interval_max_neural_footprint: u64,
    ri_reserved: [u64; 9],
}

#[cfg(target_os = "macos")]
#[allow(
    unsafe_code,
    reason = "an `unsafe extern` block is only a declaration — no call happens here; the one call site below carries its own SAFETY argument"
)]
unsafe extern "C" {
    /// `<libproc.h>`. The formal parameter type is `rusage_info_t *`
    /// (`void **`) in Apple's header; every real caller casts the address of
    /// a `struct rusage_info_vN` to it, so the value crossing the FFI
    /// boundary is a plain pointer to that struct either way — `*mut
    /// c_void` here matches what is actually passed, not the header's
    /// double-indirection spelling.
    fn proc_pid_rusage(
        pid: libc::pid_t,
        flavor: libc::c_int,
        buffer: *mut libc::c_void,
    ) -> libc::c_int;
}

/// This process's physical footprint (see [`Footprint`]). macOS only.
#[cfg(target_os = "macos")]
#[allow(
    unsafe_code,
    reason = "proc_pid_rusage has no safe wrapper in libc; m1-the-graph/57 covers this call and its soundness argument, mirroring ADR-0015's for getrusage above"
)]
pub fn footprint() -> Result<Footprint, MemoryError> {
    // Zeroed rather than left uninitialised, as `max_rss` does above: every
    // field is an integer or byte array, so all-zeroes is valid regardless
    // of which fields the kernel writes for this flavor.
    let mut usage = std::mem::MaybeUninit::<RusageInfoV4>::zeroed();
    // SAFETY: `usage.as_mut_ptr()` points at a live, correctly aligned
    // `RusageInfoV4`, owned by this frame and not aliased; its size is at
    // least `sizeof(struct rusage_info_v4)` (every real field mirrored
    // above), so the kernel's `RUSAGE_INFO_V4`-sized write lands entirely
    // inside it. `getpid()` always succeeds and `RUSAGE_INFO_V4` is a valid
    // flavor.
    let queried = unsafe {
        proc_pid_rusage(
            libc::getpid(),
            RUSAGE_INFO_V4,
            usage.as_mut_ptr().cast::<libc::c_void>(),
        )
    };
    if queried != 0 {
        return Err(MemoryError::Footprint(io::Error::last_os_error()));
    }
    // SAFETY: zero-initialised above, so it is initialised regardless of
    // what the call wrote.
    let usage = unsafe { usage.assume_init() };
    Ok(Footprint {
        current: usage.ri_phys_footprint,
        peak: usage.ri_lifetime_max_phys_footprint,
    })
}

/// This process's physical footprint. Unsupported outside macOS: see
/// [`MemoryError::Unsupported`].
#[cfg(not(target_os = "macos"))]
pub fn footprint() -> Result<Footprint, MemoryError> {
    Err(MemoryError::Unsupported)
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

    /// Same shape as `a_known_allocation_raises_the_peak_by_at_least_its_size`
    /// above, for [`footprint`] instead of [`peak_rss`]: a known allocation,
    /// larger than the current peak so it cannot be hidden under an earlier
    /// high-water mark, raises `peak` by at least (most of) its own size.
    #[test]
    #[cfg(target_os = "macos")]
    fn a_known_allocation_raises_the_footprint_peak_by_at_least_its_size() {
        // The baseline is the higher of the two readings, not `peak` alone:
        // `ri_lifetime_max_phys_footprint` is updated by the kernel
        // opportunistically rather than on every page fault, so it can
        // transiently read a touch *below* `ri_phys_footprint` right after a
        // fast allocation (seen under `cargo test`'s parallel threads:
        // current 133,267,888 B vs peak 133,251,504 B, a ~16 KiB gap). Using
        // the higher reading on both sides keeps the test honest about the
        // rise without depending on which field the kernel has caught up on.
        let baseline = footprint().unwrap();
        let before = baseline.current.max(baseline.peak);
        assert!(
            before < 512 * MIB,
            "this test allocates `peak + {SLAB}` bytes and the footprint is \
             already {before} bytes; something before it used far more memory \
             than the fast tier's budget allows"
        );

        let want = usize::try_from(before + SLAB).unwrap();
        let mut slab = vec![0u8; want];
        for page in slab.chunks_mut(4096) {
            page[0] = 1;
        }
        let touched = std::hint::black_box(&slab).len();
        let after = footprint().unwrap();

        assert_eq!(touched, want, "the whole buffer was written");
        let highest = after.current.max(after.peak);
        let least = SLAB / 4 * 3;
        let rise = highest.saturating_sub(before);
        assert!(
            rise >= least,
            "expected the footprint to rise by at least {least} bytes \
             after touching {want} of them, but it rose by {rise} ({after:?})"
        );
    }

    /// Off macOS, [`footprint`] reports why rather than a wrong number.
    #[test]
    #[cfg(not(target_os = "macos"))]
    fn footprint_is_unsupported_off_macos() {
        assert!(matches!(footprint(), Err(MemoryError::Unsupported)));
    }

    /// A measurement hands the work's own value straight back.
    #[test]
    fn measure_returns_what_the_work_returned() {
        let (answer, _) = measure(|| "the work's own value").unwrap();
        assert_eq!(answer, "the work's own value");
    }
}
