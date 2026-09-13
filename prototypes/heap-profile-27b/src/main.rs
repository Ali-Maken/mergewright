mod budget_run;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::io::Write;

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() { bump(l.size() as isize); }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }; bump(-(l.size() as isize));
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() { bump(n as isize - l.size() as isize); }
        q
    }
}
fn bump(d: isize) {
    let live = if d >= 0 { LIVE.fetch_add(d as usize, Relaxed) + d as usize } else { LIVE.fetch_sub((-d) as usize, Relaxed) - (-d) as usize };
    let mut p = PEAK.load(Relaxed);
    while live > p { match PEAK.compare_exchange_weak(p, live, Relaxed, Relaxed) { Ok(_) => break, Err(x) => p = x } }
}
#[global_allocator]
static A: Counting = Counting;
const MIB: usize = 1024 * 1024;

pub fn mark(what: &str) {
    if what == "top" && std::env::var_os("STOP_AT_TOP").is_some() {
        eprintln!("STOPPING pid {} at top", std::process::id());
        std::process::Command::new("kill").args(["-STOP", &std::process::id().to_string()]).status().unwrap();
    }
    eprintln!("MARK {what}: live {} MiB, heap peak {} MiB, rss peak {} MiB",
        LIVE.load(Relaxed) / MIB, PEAK.load(Relaxed) / MIB,
        graft_fixtures::peak_rss().unwrap() as usize / MIB);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let repo = std::path::PathBuf::from(args.next().unwrap());
    // Optional: stop self once live heap first reaches this many MiB.
    let stop_at: Option<usize> = args.next().map(|s| s.parse().unwrap()).filter(|s| *s > 0);
    let series = args.next();
    std::thread::spawn(move || {
        let mut out = series.map(|p| std::fs::File::create(p).unwrap());
        let t0 = std::time::Instant::now();
        let mut stopped = false; let mut tick = 0u64;
        loop {
            let live = LIVE.load(Relaxed);
            tick += 1;
            if let Some(f) = out.as_mut() {
                let rss = if tick % 10 == 0 { let o = std::process::Command::new("ps").args(["-o","rss=","-p",&std::process::id().to_string()]).output().unwrap(); String::from_utf8_lossy(&o.stdout).trim().parse::<usize>().unwrap_or(0) / 1024 } else { 0 };
                if rss > 0 { writeln!(f, "{} heap {} rss {}", t0.elapsed().as_millis(), live / MIB, rss).unwrap(); }
            }
            if let Some(s) = stop_at { if !stopped && live >= s * MIB {
                stopped = true;
                eprintln!("STOPPING pid {} at live {} MiB", std::process::id(), live / MIB);
                std::process::Command::new("kill").args(["-STOP", &std::process::id().to_string()]).status().unwrap();
            }}
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    });
    let n = budget_run::run(&repo).unwrap();
    eprintln!("{n:?}");
    mark("end");
}
