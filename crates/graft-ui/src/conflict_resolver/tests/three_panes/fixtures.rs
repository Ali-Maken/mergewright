//! The corpus the three-pane tests sweep, and the documents built from it
//! through `graft_git`'s own parser.

use graft_git::conflict::{ConflictFile, Projection, Region, Take, parse};

use super::super::*;

/// The files every test here sweeps: two one-line blocks (the capture's
/// `notes.txt`), theirs longer than ours, ours longer than theirs, an
/// empty ours, a block with a base, blocks first and last, and two blocks
/// back to back.
pub(super) const CORPUS: [&[u8]; 7] = [
    b"one\n<<<<<<< HEAD\ntwo from main\n=======\nTWO from feature\n>>>>>>> feature\nthree\nfour\n<<<<<<< HEAD\nfive from main\n=======\nFIVE from feature\n>>>>>>> feature\nsix\n",
    b"a\n<<<<<<< HEAD\nb\n=======\nB1\nB2\nB3\n>>>>>>> t\nc\n<<<<<<< HEAD\nd\n=======\nD\n>>>>>>> t\ne\n",
    b"a\n<<<<<<< HEAD\nb1\nb2\nb3\n=======\nB\n>>>>>>> t\nc\n<<<<<<< HEAD\nd\n=======\nD\n>>>>>>> t\n",
    b"a\n<<<<<<< HEAD\n=======\nadded\n>>>>>>> t\nb\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> t\n",
    b"a\n<<<<<<< HEAD\nours\n||||||| base\nold one\nold two\nold three\n=======\ntheirs\n>>>>>>> t\nb\n<<<<<<< HEAD\nc\n||||||| base\nC\n=======\nd\n>>>>>>> t\n",
    b"<<<<<<< HEAD\nfirst\n=======\nFIRST\nFIRST TOO\n>>>>>>> t\nmiddle\n<<<<<<< HEAD\nlast\n=======\nLAST\n>>>>>>> t\n",
    b"<<<<<<< HEAD\na\n=======\nA\n>>>>>>> t\n<<<<<<< HEAD\nb\nb\n=======\nB\n>>>>>>> t\nend\n",
];

/// One line of a parsed file, as the app hands it to the widget.
fn text_of(line: &graft_git::conflict::Line) -> SanitizedText {
    text(String::from_utf8_lossy(&line.0).trim_end_matches('\n'))
}

/// `file` as the app's `view::resolver::regions` builds it.
pub(super) fn doc_of(file: &ConflictFile, base_shown: bool) -> DocView {
    let [ours, theirs, result] =
        [Projection::Ours, Projection::Theirs, Projection::Result].map(|p| file.first_lines(p));
    let of = file.blocks().len();
    let mut index = 0;
    let regions = file
        .regions
        .iter()
        .enumerate()
        .map(|(r, region)| {
            let numbers = BlockNumbers {
                ours: ours[r],
                theirs: theirs[r],
                result: result[r],
            };
            let (block, state) = match region {
                Region::Plain(lines) => {
                    return RegionView::Plain {
                        numbers,
                        lines: lines.iter().map(text_of).collect(),
                    };
                }
                Region::Conflict(block) => (block, BlockState::Open),
                Region::Resolved { block, take } => (
                    block,
                    BlockState::Took(match take {
                        Take::Ours => TakeView::Ours,
                        Take::Theirs => TakeView::Theirs,
                        Take::Both => TakeView::Both,
                    }),
                ),
            };
            index += 1;
            RegionView::Block(BlockView {
                index: index - 1,
                of,
                state,
                ours: block.ours.iter().map(text_of).collect(),
                base: block
                    .shown_base()
                    .map(|(base, _)| (text("base"), base.lines.iter().map(text_of).collect())),
                theirs: block.theirs.iter().map(text_of).collect(),
                first_line: numbers.result,
                numbers,
            })
        })
        .collect();
    DocView {
        title: text("notes.txt · 2 conflicts · 2 open"),
        files: FileListView::default(),
        names: names(),
        regions,
        cursor_block: Some(0),
        base_shown,
        notice: None,
        finish: None,
    }
}

/// The states each file is swept through.
#[derive(Clone, Copy, Debug)]
pub(super) enum Sweep {
    NothingTaken,
    Block0Ours,
    AllTheirs,
    Block1Both,
    Block0OpenWithBase,
}

pub(super) const SWEEPS: [Sweep; 5] = [
    Sweep::NothingTaken,
    Sweep::Block0Ours,
    Sweep::AllTheirs,
    Sweep::Block1Both,
    Sweep::Block0OpenWithBase,
];

/// `bytes` parsed and taken as `sweep` says: the file, and its doc.
pub(super) fn swept(bytes: &[u8], sweep: Sweep) -> (ConflictFile, DocView) {
    let mut file = parse(bytes).expect("a corpus file parses");
    let blocks = file.blocks().len();
    let take = |file: &mut ConflictFile, block: usize, take: Take| {
        file.take(block, take).expect("the corpus block exists");
    };
    match sweep {
        Sweep::NothingTaken | Sweep::Block0OpenWithBase => {}
        Sweep::Block0Ours => take(&mut file, 0, Take::Ours),
        Sweep::AllTheirs => (0..blocks).for_each(|b| take(&mut file, b, Take::Theirs)),
        Sweep::Block1Both => take(&mut file, 1, Take::Both),
    }
    let doc = doc_of(&file, matches!(sweep, Sweep::Block0OpenWithBase));
    (file, doc)
}
