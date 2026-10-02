//! The Unicode tier's per-frame cost under `criterion` (PROMPT.md §6.3,
//! ticket m1-the-graph/30): one screen of graph rows drawn into a ratatui
//! buffer by `GraphColumn`.
//!
//! Review-only (PROMPT.md §7.1), with short runs:
//! `cargo bench -p graft-ui -- --warm-up-time 1 --measurement-time 3`.

#![allow(
    clippy::expect_used,
    reason = "a benchmark whose input cannot be built has nothing to measure"
)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Rgb,
    Supported, Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::identity::BranchIdentity;
use graft_graph::layout::layout;
use graft_graph::seeds::{BranchSeed, Seeds};
use graft_ui::graph_column::{GraphColumn, GraphLine};
use graft_ui::theme::{Theme, ThemeKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// One tall terminal's worth of graph rows.
const SCREEN_ROWS: usize = 60;
/// Wide enough for every lane the generated history opens.
const COLUMNS: u16 = 40;

fn oid(index: usize) -> Oid {
    let mut bytes = [0u8; Oid::SHA1_BYTES];
    bytes[..8].copy_from_slice(&(index as u64 + 1).to_be_bytes());
    Oid::from_bytes(&bytes).expect("20 bytes is a valid oid length")
}

fn at(seconds: i64) -> Identity {
    Identity {
        name: b"Gen".to_vec(),
        email: b"gen@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    }
}

/// A first-parent chain with a side commit every fifth row merged back three
/// rows up (the shape `graft-graph`'s strip-cache tests use).
fn commits(count: usize) -> Vec<Commit> {
    (0..count)
        .map(|index| {
            let mut parents = Vec::new();
            if index + 1 < count {
                parents.push(oid(index + 1));
            }
            if index % 5 == 0 && index + 3 < count {
                parents.push(oid(index + 3));
            }
            let seconds = 2_000_000_000 - i64::try_from(index).expect("small index") * 60;
            Commit {
                id: oid(index),
                parents,
                author: at(seconds),
                committer: at(seconds),
                message: format!("commit {index}").into_bytes(),
            }
        })
        .collect()
}

/// A truecolour terminal that answered every probe.
fn terminal() -> Capabilities {
    Capabilities {
        tier: RenderingTier::Unicode,
        pixel_protocol: None,
        kitty_graphics: Supported::No,
        kitty_deflate: false,
        answers_dsr: false,
        sixel: Supported::No,
        iterm2_images: false,
        kitty_keyboard: Supported::No,
        synchronized_output: Supported::Yes,
        cell_size: CellSize {
            width: 9,
            height: 18,
            source: CellSizeSource::Reported,
        },
        theme: ThemeHint {
            theme: TerminalTheme::Dark,
            source: ThemeSource::Background(Rgb {
                red: 0x10,
                green: 0x10,
                blue: 0x14,
            }),
        },
        color_depth: ColorDepth::TrueColor,
        no_color: false,
        is_tty: true,
        over_ssh: false,
        tmux: TmuxStatus::Outside,
        probe: ProbeOutcome::Answered,
        probe_budget: Duration::from_millis(250),
    }
}

fn row_render(c: &mut Criterion) {
    let seeds = Seeds {
        default_branch: Some(BranchSeed {
            tip: oid(0),
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    };
    let rows = layout(&commits(1_000), seeds);
    let theme = Theme::new(ThemeKind::Dark, &terminal());
    let lines: Vec<GraphLine<'_>> = rows[500..500 + SCREEN_ROWS]
        .iter()
        .map(GraphLine::plain)
        .collect();
    let area = Rect::new(
        0,
        0,
        COLUMNS,
        u16::try_from(SCREEN_ROWS).expect("a screen of rows fits a u16"),
    );

    c.bench_function("GraphColumn::render/one screen", |b| {
        b.iter(|| {
            let mut buffer = Buffer::empty(area);
            GraphColumn::new(&lines, &theme).render(area, &mut buffer);
            black_box(buffer)
        });
    });
}

criterion_group!(benches, row_render);
criterion_main!(benches);
