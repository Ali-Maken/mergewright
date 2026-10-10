//! The drain's own tests: what it reads of the stream, and when.

use super::*;

fn wait_quiet(sink: &DrainSink) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !sink.quiet_for(Duration::from_millis(20)) {
        assert!(Instant::now() < deadline, "the drain never went quiet");
        thread::sleep(Duration::from_millis(5));
    }
}

/// 1 KiB pipe at 40 KB/s, 20 KB written: the writer is held back to the
/// rate (±10%), and `written`, `drained_at`, `quiet_for` agree.
#[test]
fn drains_at_its_rate() {
    const RATE: u64 = 40_000;
    const TOTAL: usize = 20_000;
    let sink = DrainSink::new(1024, RATE);
    let mut writer = sink.writer();
    assert!(sink.quiet_for(Duration::ZERO), "empty at the start");

    let started = Instant::now();
    let first = writer
        .write(&vec![b'x'; 4096])
        .unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(first, 1024, "a write takes only the room there is");
    assert_eq!(sink.written(), 1024);
    assert!(!sink.quiet_for(Duration::ZERO), "full, not quiet");
    writer
        .write_all(&vec![b'x'; TOTAL - 1024])
        .unwrap_or_else(|why| panic!("{why}"));
    let wrote = started.elapsed().as_millis();
    assert_eq!(sink.written(), TOTAL as u64);
    // The writer returns once the last byte is in the pipe: TOTAL minus
    // one pipeful had to drain first.
    let at_rate = |bytes: usize| bytes as u128 * 1000 / u128::from(RATE);
    let within = |took: u128, expected: u128| took.abs_diff(expected) * 10 <= expected;
    let expected = at_rate(TOTAL - 1024);
    assert!(
        within(wrote, expected),
        "writer held {wrote} ms, rate says {expected} ms"
    );

    wait_quiet(&sink);
    let start = sink.drained_at(1).unwrap_or_else(|| panic!("first byte"));
    let end = sink
        .drained_at(TOTAL as u64)
        .unwrap_or_else(|| panic!("last byte drained"));
    assert!(
        sink.drained_at(TOTAL as u64 + 1).is_none(),
        "nothing past the end"
    );
    let drained = end.duration_since(start).as_millis();
    let expected = at_rate(TOTAL);
    assert!(
        within(drained, expected),
        "drained in {drained} ms, rate says {expected} ms"
    );
    assert!(sink.kitty_transmits().is_empty());
}

#[test]
fn counts_kitty_transmits_across_writes() {
    let payload = "QUJD".repeat(5000);
    let stream = [
        format!("text\x1b[31m\x1b_Ga=T,f=32,i=7,m=1;{payload}\x1b\\"),
        format!("\x1b_Gm=1;{payload}\x1b\\\x1b_Gm=0;{payload}\x1b\\"),
        "\x1b_Ga=p,i=7,p=1\x1b\\\x1b_Ga=d,d=i,i=7\x1b\\".to_owned(),
        format!("\x1b_Gf=100,i=2147483657;{payload}\x1b\\"),
        format!("\x1b_Ga=t,f=32,o=z,i=9,s=34,v=37,q=2;{payload}\x1b\\ tail"),
    ]
    .concat()
    .into_bytes();
    let starts: Vec<u64> = stream
        .windows(3)
        .enumerate()
        .filter(|(_, w)| *w == b"\x1b_G")
        .map(|(at, _)| at as u64)
        .collect();
    // Fast drain; a pipe smaller than the payload so writes split.
    let sink = DrainSink::new(4096, 50_000_000);
    let mut writer = sink.writer();
    // Split inside the first header (after `ESC _`, inside `i=7`).
    let cuts = [0, 11, 22, 30, stream.len()];
    for pair in cuts.windows(2) {
        writer
            .write_all(&stream[pair[0]..pair[1]])
            .unwrap_or_else(|why| panic!("{why}"));
    }
    assert_eq!(sink.written(), stream.len() as u64);
    let found = sink.kitty_transmits();
    assert_eq!(
        found,
        vec![
            KittyTransmit {
                offset: starts[0],
                image_id: 7,
                width_px: None,
                height_px: None,
                deflated: false,
            },
            KittyTransmit {
                offset: starts[5],
                image_id: 2_147_483_657,
                width_px: None,
                height_px: None,
                deflated: false,
            },
            KittyTransmit {
                offset: starts[6],
                image_id: 9,
                width_px: Some(34),
                height_px: Some(37),
                deflated: true,
            },
        ],
        "a=T, no-a first chunk, a=t; not m= continuations, a=p, a=d; s=, v= and o=z read"
    );
    wait_quiet(&sink);
}

/// A fence is answered once the drain has passed its last byte, never
/// when written; one cut across two writes is still one fence; a cursor
/// move that starts like one is not.
#[test]
fn answers_each_fence_once_drained_past() {
    let sink = DrainSink::new(64, 20_000);
    let answers = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&answers);
    sink.answering_fences(move || {
        log.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Instant::now());
    });
    let mut writer = sink.writer();
    let filler = "x".repeat(400);
    let stream = [
        format!("{filler}\x1b[5;1H\x1b["),
        format!("5n{filler}"),
        "\x1b[5n".to_owned(),
    ];
    let mut ends = Vec::new();
    for part in &stream {
        writer
            .write_all(part.as_bytes())
            .unwrap_or_else(|why| panic!("{why}"));
        if part.contains("5n") {
            ends.push(sink.written());
        }
    }
    let ends = [ends[0] - filler.len() as u64, ends[1]];
    wait_quiet(&sink);
    assert_eq!(sink.fences_written(), 2);
    let answered = answers
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(answered.len(), 2, "one answer per fence");
    for (at, end) in answered.iter().zip(ends) {
        let passed = sink
            .drained_at(end)
            .unwrap_or_else(|| panic!("drained past {end}"));
        assert!(*at >= passed, "answered only once read");
    }
}
