//! Throwaway (m1-the-graph/59): live heap vs footprint per no-cg stage.
//! Live = bytes Rust holds (counting allocator); footprint - live = what
//! malloc keeps dirty after free, plus non-Rust allocations.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use graft::history::{Promote, View, Views, WalkSource, seeds_for, topological_order};
use graft::repos::RepoStore;
use graft_core::commit_handle::CommitHandle;
use graft_git::history::{Order, Walk};
use graft_graph::layout::stream::{Anchor, StreamConfig};

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
        unsafe { System.dealloc(p, l) };
        bump(-(l.size() as isize));
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

#[repr(C)]
#[derive(Default)]
struct ZoneStats { blocks_in_use: u32, size_in_use: usize, max_size_in_use: usize, size_allocated: usize }
unsafe extern "C" {
    fn malloc_zone_statistics(zone: *mut std::ffi::c_void, stats: *mut ZoneStats);
    fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
}
fn zone() -> (usize, usize) {
    let mut z = ZoneStats::default();
    unsafe { malloc_zone_statistics(std::ptr::null_mut(), &mut z) };
    (z.size_in_use, z.size_allocated)
}
/// Discriminator: if footprint falls to ~live after this, the gap was free
/// pages malloc kept, not fragmentation.
fn relief(what: &str) {
    let freed = unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
    let f = graft_fixtures::footprint().unwrap();
    println!("  relief after {what}: returned {:.1} MiB, footprint now {:.1}", mib(freed), mib(f.current as usize));
}
fn mib(b: usize) -> f64 { b as f64 / 1048576.0 }
fn mark(what: &str) {
    let f = graft_fixtures::footprint().unwrap();
    let (in_use, allocated) = zone();
    println!("{what:<20} live {:>7.1}  live-peak-in-stage {:>7.1}  footprint {:>7.1} / peak {:>7.1}  zone in_use {:>7.1} allocated {:>7.1}",
        mib(LIVE.load(Relaxed)), mib(PEAK.load(Relaxed)), mib(f.current as usize), mib(f.peak as usize), mib(in_use), mib(allocated));
    PEAK.store(LIVE.load(Relaxed), Relaxed);
}

fn main() {
    let repo = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let never = || false;
    mark("start");
    let store = RepoStore::new(repo, None);
    let table = store.table().unwrap();
    let refs = store.refs().unwrap();
    let branches = store.branches().unwrap();
    let shared = store.shared().unwrap();
    let handle = shared.handle();
    let seeds = seeds_for(&refs.refs, &branches);
    mark("open");
    let walk = Walk::from_tips(&handle, Order::Topological, &refs.tips, Arc::clone(&table)).unwrap();
    let h = &handle; let t = Arc::clone(&table);
    let mut started = Some(walk);
    let open = move |promoted: Option<&Arc<[CommitHandle]>>| {
        if let Some(order) = promoted { return Ok(WalkSource::in_order(h, Arc::clone(&t), Arc::clone(order))); }
        Ok(WalkSource::new(h, started.take().unwrap()))
    };
    let mut views = Views::new(seeds, StreamConfig::default(), Box::new(open));
    let view = View::default();
    let layout = views.layout(&view).unwrap();
    layout.set_viewport(0..40);
    layout.advance(500, &never).unwrap();
    layout.rows(0..40).unwrap();
    mark("first paint");
    while !layout.is_done() { layout.advance(usize::MAX, &never).unwrap(); }
    let rows = layout.rows_emitted();
    layout.set_viewport(rows - 40..rows);
    layout.rows(rows - 40..rows).unwrap();
    let last = layout.id_at(rows - 1).unwrap();
    mark("date walk to end");
    println!("  interner {:.1} MiB, order {:.1} MiB", mib(table.approximate_bytes()), mib(layout.resident().order_bytes));
    if std::env::var_os("RELIEF").is_some() { relief("date walk"); }
    let order = topological_order(&store, &never).unwrap().unwrap();
    mark("topological_order");
    if std::env::var_os("RELIEF").is_some() { relief("sort"); }
    let promote = Promote { order, anchor: Some(Anchor { commit: last, offset: 39 }), height: 40 };
    assert!(views.promote(&view, &promote, &never).unwrap());
    mark("promote swap");
    let layout = views.layout(&view).unwrap();
    layout.set_viewport(0..40);
    layout.rows(0..40).unwrap();
    mark("g");
    drop(views); drop(promote);
    mark("views dropped");
    if std::env::var_os("RELIEF").is_some() { relief("end"); }
}
