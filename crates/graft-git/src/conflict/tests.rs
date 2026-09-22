//! The parse rules, checked against files git itself wrote (the stopped
//! corpus shapes) and against hand-written edges git never writes.

use std::path::{Path, PathBuf};

use graft_fixtures::Shape;

use super::{
    BaseMismatch, Block, BlockState, ConflictFile, Diff3Error, Line, MarkerEnds, ParseError,
    ParseErrorKind, ReadError, Region, Take, WriteError, attach_base, blob_id, diff3_file, parse,
    read, stage_bytes, write_back,
};
use crate::history_reads::first_marker_line;
use crate::repo::RepoHandle;

/// The corpus files git wrote markers into, in both stopped shapes.
const MARKER_FILES: &[&str] = &[
    "both.txt",
    "three.txt",
    "crlf.txt",
    "nofinal.txt",
    "md.md",
    "zdiff3.txt",
    "both-added.txt",
];

fn borrowed(shape: Shape) -> PathBuf {
    graft_fixtures::borrow(shape)
}

fn open(dir: &Path) -> RepoHandle {
    RepoHandle::open(dir).expect("the fixture opens")
}

fn bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|why| panic!("reading {}: {why}", path.display()))
}

fn git(dir: &Path, args: &[&str]) -> Vec<u8> {
    graft_fixtures::git::run(dir, args).unwrap_or_else(|why| panic!("{why}"))
}

fn lines(text: &[&[u8]]) -> Vec<Line> {
    text.iter().map(|line| Line(line.to_vec())).collect()
}

fn one_block(file: &ConflictFile) -> &Block {
    let blocks: Vec<&Block> = file
        .regions
        .iter()
        .filter_map(|region| match region {
            Region::Conflict(block) => Some(block),
            _ => None,
        })
        .collect();
    assert_eq!(blocks.len(), 1, "exactly one block in {file:?}");
    blocks[0]
}

#[test]
fn round_trip_is_byte_exact() {
    let dir = borrowed(Shape::StoppedRebase);
    for name in MARKER_FILES {
        let input = bytes(&dir.join(name));
        let file = parse(&input).unwrap_or_else(|why| panic!("{name}: {why}"));
        assert_eq!(file.render(), input, "{name} does not round-trip");
        assert!(file.open() >= 1, "{name} should have a block");
    }

    let hand_written: &[&[u8]] = &[
        b"",
        b"plain\n",
        b"plain, no final newline",
        b"a\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> feature\nb\n",
        b"a\r\n<<<<<<< HEAD\r\nours\r\n=======\r\ntheirs\r\n>>>>>>> feature\r\nb\r\n",
        // Mixed: CRLF markers, LF content, and a CR-less last line.
        b"a\n<<<<<<< HEAD\r\nours\n||||||| base\r\nbase\n=======\r\ntheirs\r\n>>>>>>> feature\nb",
        // The end marker is the file's last, newline-less line.
        b"<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> feature",
        // Labels with spaces, and an empty one (git writes `<<<<<<< ` then).
        b"<<<<<<< 3f2a1b (Add OAuth provider)\nx\n||||||| parent of 3f2a1b (Add OAuth provider)\ny\n=======\nz\n>>>>>>> \n",
        // Empty sides.
        b"<<<<<<< a\n=======\n>>>>>>> b\n",
        // Two blocks back to back, nothing between.
        b"<<<<<<< a\n1\n=======\n2\n>>>>>>> b\n<<<<<<< a\n3\n=======\n4\n>>>>>>> b\n",
        // A lone `\r` inside content is content.
        b"<<<<<<< a\nx\ry\n=======\n\r\n>>>>>>> b\n",
    ];
    for input in hand_written {
        let file = parse(input).unwrap_or_else(|why| panic!("{input:?}: {why}"));
        assert_eq!(file.render(), *input, "{input:?} does not round-trip");
    }
}

/// What `git merge-file -p -L ours -L base -L theirs` printed on 2026-09-22
/// (git 2.54) for `md.md`'s three versions — the ruling's evidence, kept
/// where the parser is tested against it.
const MERGE_FILE_MD: &[u8] = b"Title\n=====\n\n<<<<<<< ours\nIntro line ours\n=======\nHeading\n=======\n\nIntro line theirs\n>>>>>>> theirs\n";

#[test]
fn setext_underline_after_the_separator_is_content() {
    let printed = parse(MERGE_FILE_MD).expect("git's own output parses");
    let block = one_block(&printed);
    assert_eq!(block.ours, lines(&[b"Intro line ours\n"]));
    assert_eq!(
        block.theirs,
        lines(&[b"Heading\n", b"=======\n", b"\n", b"Intro line theirs\n"])
    );
    assert_eq!(printed.render(), MERGE_FILE_MD);

    // The same through a real stop: the corpus file, with git's labels.
    let dir = borrowed(Shape::StoppedRebase);
    let on_disk = parse(&bytes(&dir.join("md.md"))).expect("md.md parses");
    let block = one_block(&on_disk);
    assert_eq!(block.ours_label, b"HEAD");
    assert!(
        block.theirs.contains(&Line(b"=======\n".to_vec())),
        "theirs should hold the underline as content: {:?}",
        block.theirs
    );
    assert_eq!(on_disk.open(), 1);
}

#[test]
fn diff3_and_zdiff3_carry_base() {
    let dir = borrowed(Shape::StoppedRebase);
    let zdiff3 = parse(&bytes(&dir.join("zdiff3.txt"))).expect("zdiff3.txt parses");
    assert!(zdiff3.open() >= 1);
    for region in &zdiff3.regions {
        if let Region::Conflict(block) = region {
            let (label, base) = block
                .base
                .as_ref()
                .expect("a zdiff3 block carries its base");
            assert_eq!(label, b"base");
            assert_eq!(base, &lines(&[b"middle\n"]));
        }
    }
    let plain_style = parse(&bytes(&dir.join("both.txt"))).expect("both.txt parses");
    assert!(one_block(&plain_style).base.is_none());
}

#[test]
fn take_matches_git_checkout_sides() {
    let scratch = graft_fixtures::copy_into_temp(Shape::StoppedMerge);
    let dir = scratch.path();
    for name in [
        "both.txt",
        "crlf.txt",
        "md.md",
        "nofinal.txt",
        "both-added.txt",
    ] {
        let path = dir.join(name);
        let conflicted = bytes(&path);
        git(dir, &["checkout", "--ours", "--", name]);
        let ours = bytes(&path);
        git(dir, &["checkout", "--theirs", "--", name]);
        let theirs = bytes(&path);

        let mut file = parse(&conflicted).expect("the conflicted file parses");
        one_block(&file);
        file.take(0, Take::Ours).expect("block 0 exists");
        assert_eq!(file.render(), ours, "{name}: take ours");
        file.take(0, Take::Theirs).expect("re-taking replaces");
        assert_eq!(file.render(), theirs, "{name}: take theirs");
        file.take(0, Take::Both).expect("re-taking replaces");
        assert_eq!(
            file.render(),
            both_sides(&ours, &theirs),
            "{name}: take both is ours then theirs"
        );
        assert_eq!(file.blocks()[0].state, BlockState::Took(Take::Both));
        assert_eq!(file.open(), 0);
    }
}

/// What taking both sides of a one-block file must give, built from git's
/// two checkouts alone: the lines they share at the start, ours' own lines,
/// theirs' own lines, the lines they share at the end.
fn both_sides(ours: &[u8], theirs: &[u8]) -> Vec<u8> {
    let ours: Vec<&[u8]> = ours.split_inclusive(|byte| *byte == b'\n').collect();
    let theirs: Vec<&[u8]> = theirs.split_inclusive(|byte| *byte == b'\n').collect();
    let shortest = ours.len().min(theirs.len());
    let prefix = ours.iter().zip(&theirs).take_while(|(a, b)| a == b).count();
    let suffix = ours
        .iter()
        .rev()
        .zip(theirs.iter().rev())
        .take(shortest - prefix)
        .take_while(|(a, b)| a == b)
        .count();
    let mut expected = Vec::new();
    for line in ours[..ours.len() - suffix]
        .iter()
        .chain(&theirs[prefix..theirs.len() - suffix])
        .chain(&ours[ours.len() - suffix..])
    {
        expected.extend_from_slice(line);
    }
    expected
}

#[test]
fn write_back_refuses_a_changed_file() {
    let scratch = graft_fixtures::copy_into_temp(Shape::StoppedRebase);
    let path = scratch.path().join("both.txt");
    let before = bytes(&path);
    let mut file = parse(&before).expect("both.txt parses");
    file.take(0, Take::Theirs).expect("block 0 exists");

    let stale = blob_id(b"not what is on disk");
    match write_back(&path, &file, stale) {
        Err(WriteError::Changed { now }) => assert_eq!(now, blob_id(&before)),
        other => panic!("a stale expectation must be refused, got {other:?}"),
    }
    assert_eq!(
        bytes(&path),
        before,
        "a refused write leaves the file alone"
    );
}

#[cfg(unix)]
#[test]
fn write_back_keeps_mode_and_leaves_no_temp() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = graft_fixtures::copy_into_temp(Shape::StoppedRebase);
    let path = scratch.path().join("both.txt");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let before = bytes(&path);
    let mut file = parse(&before).expect("both.txt parses");
    file.take(0, Take::Ours).expect("block 0 exists");

    let new_id = write_back(&path, &file, file.fingerprint).expect("the right expectation writes");
    let after = bytes(&path);
    assert_eq!(after, file.render());
    assert_eq!(new_id, blob_id(&after));
    assert_eq!(
        std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777,
        0o755,
        "mode bits survive the rename"
    );
    let leftovers: Vec<String> = std::fs::read_dir(scratch.path())
        .expect("the directory lists")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with(".graft-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temporaries left behind: {leftovers:?}"
    );
    // The second write sees the new fingerprint, not the old one.
    assert!(matches!(
        write_back(&path, &file, file.fingerprint),
        Err(WriteError::Changed { .. })
    ));
}

#[test]
fn blob_id_is_gits() {
    let dir = borrowed(Shape::Trivial);
    let one_mib: Vec<u8> = (0..1024 * 1024u32)
        .map(|i| u8::try_from((i * 7 + i / 251) % 256).expect("in range"))
        .collect();
    for input in [&b""[..], b"x", &one_mib] {
        let printed =
            graft_fixtures::git::run_isolated(&dir, &["hash-object", "--stdin"], Some(input))
                .unwrap_or_else(|why| panic!("{why}"));
        let hex = String::from_utf8_lossy(&printed).trim().to_owned();
        assert_eq!(blob_id(input).to_string(), hex, "for {} bytes", input.len());
    }
}

#[test]
fn read_names_binary_and_too_large() {
    let dir = borrowed(Shape::StoppedRebase);
    assert!(matches!(read(&dir.join("pic.bin")), Err(ReadError::Binary)));

    let scratch = tempfile::tempdir().expect("a scratch directory");
    let big = scratch.path().join("big.txt");
    let nine_mib = 9 * 1024 * 1024u64;
    let file = std::fs::File::create(&big).expect("create");
    file.set_len(nine_mib).expect("a sparse 9 MiB file");
    drop(file);
    match read(&big) {
        Err(ReadError::TooLarge { bytes }) => assert_eq!(bytes, nine_mib),
        other => panic!("a 9 MiB file must be refused as too large, got {other:?}"),
    }

    let missing = read(&scratch.path().join("absent.txt"));
    assert!(matches!(missing, Err(ReadError::Io(_))));
    let plain = read(&dir.join("added-by-us.txt")).expect("a plain file reads");
    assert_eq!(plain.open(), 0);
}

#[test]
fn stage_bytes_reads_and_names_missing() {
    let dir = borrowed(Shape::StoppedRebase);
    let repo = open(&dir);
    let path = Path::new("theirs-deleted.txt");
    assert_eq!(stage_bytes(&repo, path, 3).expect("stage 3 answers"), None);
    let ours = stage_bytes(&repo, path, 2)
        .expect("stage 2 answers")
        .expect("stage 2 is there");
    assert_eq!(ours, git(&dir, &["show", ":2:theirs-deleted.txt"]));
    let base = stage_bytes(&repo, path, 1)
        .expect("stage 1 answers")
        .expect("stage 1 is there");
    assert_eq!(base, b"kept by us, deleted by them\n");
    assert!(
        stage_bytes(&repo, path, 0).is_err(),
        "stage 0 is not a stage"
    );
    assert_eq!(
        stage_bytes(&repo, Path::new("not-in-the-index.txt"), 2).expect("answers"),
        None
    );
}

/// The base lines of every block that has one, in order.
fn attached_bases(file: &ConflictFile) -> Vec<Vec<Line>> {
    file.regions
        .iter()
        .filter_map(|region| match region {
            Region::Conflict(block) | Region::Resolved { block, .. } => {
                block.base.as_ref().map(|(_, base)| base.clone())
            }
            Region::Plain(_) => None,
        })
        .collect()
}

/// Edits the `index`-th block's ours side by hand.
fn hand_edit(file: &mut ConflictFile, index: usize) {
    let block = file
        .regions
        .iter_mut()
        .filter_map(|region| match region {
            Region::Conflict(block) | Region::Resolved { block, .. } => Some(block),
            Region::Plain(_) => None,
        })
        .nth(index)
        .expect("the block exists");
    block.ours.push(Line(b"typed by hand\n".to_vec()));
}

fn three_txt_diff3(repo: &RepoHandle) -> ConflictFile {
    let diff3 = diff3_file(repo, Path::new("three.txt"))
        .expect("merge-file runs")
        .expect("every stage is there");
    assert_eq!(diff3.open(), 3);
    diff3
}

/// `git merge-file -p` (git 2.54, 2026-09-22) for a base `alpha, one,
/// bravo, two, charlie, three, delta` whose sides each change `one`, `two`
/// and `three`: git's default style joins the three conflicts, one plain
/// line apart, into one block; `--diff3` keeps three. The evidence for
/// [`attach_base`]'s run alignment, kept verbatim.
const MERGE_STYLE_JOINED: &[u8] = b"alpha\n<<<<<<< ours\none-ours\nbravo\ntwo-ours\ncharlie\nthree-ours\n=======\none-theirs\nbravo\ntwo-theirs\ncharlie\nthree-theirs\n>>>>>>> theirs\ndelta\n";
const DIFF3_STYLE_SEPARATE: &[u8] = b"alpha\n<<<<<<< ours\none-ours\n||||||| base\none\n=======\none-theirs\n>>>>>>> theirs\nbravo\n<<<<<<< ours\ntwo-ours\n||||||| base\ntwo\n=======\ntwo-theirs\n>>>>>>> theirs\ncharlie\n<<<<<<< ours\nthree-ours\n||||||| base\nthree\n=======\nthree-theirs\n>>>>>>> theirs\ndelta\n";

#[test]
fn a_joined_block_gets_the_base_of_its_whole_run() {
    let diff3 = parse(DIFF3_STYLE_SEPARATE).expect("git's diff3 output parses");
    assert_eq!(diff3.open(), 3);

    let mut file = parse(MERGE_STYLE_JOINED).expect("git's merge output parses");
    assert_eq!(file.open(), 1);
    assert_eq!(attach_base(&mut file, &diff3), Ok(1));
    assert_eq!(
        attached_bases(&file),
        vec![lines(&[
            b"one\n",
            b"bravo\n",
            b"two\n",
            b"charlie\n",
            b"three\n"
        ])]
    );
    let Region::Conflict(block) = &file.regions[1] else {
        panic!("region 1 is the block: {:?}", file.regions);
    };
    assert_eq!(
        block.base.as_ref().map(|(label, _)| label.as_slice()),
        Some(&b"base"[..])
    );
    assert_eq!(
        file.render(),
        b"alpha\n<<<<<<< ours\none-ours\nbravo\ntwo-ours\ncharlie\nthree-ours\n||||||| base\none\nbravo\ntwo\ncharlie\nthree\n=======\none-theirs\nbravo\ntwo-theirs\ncharlie\nthree-theirs\n>>>>>>> theirs\ndelta\n"
    );

    // A run must match on both sides: theirs edited by hand, ours intact,
    // is still a mismatch.
    let mut edited = parse(MERGE_STYLE_JOINED).expect("parses");
    let Region::Conflict(block) = &mut edited.regions[1] else {
        panic!("region 1 is the block");
    };
    block.theirs.pop();
    assert_eq!(
        attach_base(&mut edited, &diff3),
        Err(BaseMismatch { block: 0 })
    );
    assert!(attached_bases(&edited).is_empty());
}

#[test]
fn base_attaches_by_equality_only() {
    let scratch = graft_fixtures::copy_into_temp(Shape::StoppedRebase);
    let dir = scratch.path();
    let repo = open(dir);
    let path = dir.join("three.txt");
    let diff3 = three_txt_diff3(&repo);

    let mut file = read(&path).expect("three.txt reads");
    assert_eq!(attach_base(&mut file, &diff3), Ok(3));
    assert_eq!(
        attached_bases(&file),
        vec![lines(&[b"one\n"]), lines(&[b"two\n"]), lines(&[b"three\n"])]
    );
    let Region::Conflict(middle) = &file.regions[3] else {
        panic!("region 3 is the middle block: {:?}", file.regions);
    };
    assert_eq!(middle.ours, lines(&[b"two-ours\n"]));
    assert_eq!(middle.theirs, lines(&[b"two-theirs\n"]));

    // In memory: a take keeps its block intact, a hand edit changes block
    // 1's lines — no reproduced block equals it, and nothing is attached.
    let mut edited = read(&path).expect("three.txt reads");
    edited.take(0, Take::Ours).expect("block 0 exists");
    hand_edit(&mut edited, 1);
    assert_eq!(
        attach_base(&mut edited, &diff3),
        Err(BaseMismatch { block: 1 })
    );
    assert!(
        attached_bases(&edited).is_empty(),
        "nothing is attached on a mismatch"
    );

    // A whole-file shape has no blocks to reproduce.
    assert!(matches!(
        diff3_file(&repo, Path::new("theirs-deleted.txt")),
        Ok(None)
    ));
    assert!(matches!(
        diff3_file(&repo, Path::new("not-in-the-index.txt")),
        Ok(None)
    ));
    let _: Option<Diff3Error> = None;
}

#[test]
fn base_survives_a_written_take_but_not_a_hand_edit() {
    let scratch = graft_fixtures::copy_into_temp(Shape::StoppedRebase);
    let dir = scratch.path();
    let repo = open(dir);
    let path = dir.join("three.txt");
    let diff3 = three_txt_diff3(&repo);

    // The take written back removes block 0 from the file while the stages
    // — and so `diff3` — still have it; the other two blocks keep their base.
    let mut taken = read(&path).expect("three.txt reads");
    taken.take(0, Take::Theirs).expect("block 0 exists");
    write_back(&path, &taken, taken.fingerprint).expect("writes");
    let mut reread = read(&path).expect("re-reads");
    assert_eq!(reread.open(), 2);
    assert_eq!(attach_base(&mut reread, &diff3), Ok(2));
    assert_eq!(
        attached_bases(&reread),
        vec![lines(&[b"two\n"]), lines(&[b"three\n"])]
    );

    // A hand edit in what is now block 1 is a mismatch all the same.
    let mut reread = read(&path).expect("re-reads");
    hand_edit(&mut reread, 1);
    assert_eq!(
        attach_base(&mut reread, &diff3),
        Err(BaseMismatch { block: 1 })
    );
    assert!(attached_bases(&reread).is_empty());
}

#[test]
fn parse_never_panics_on_marker_soup() {
    // A fixed xorshift, so a failure reproduces.
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let pieces: &[&[u8]] = &[
        b"<<<<<<< ",
        b"<<<<<<<",
        b"||||||| ",
        b"=======",
        b"======",
        b">>>>>>> ",
        b">>>>>>>",
        b"HEAD",
        b"x",
        b"\n",
        b"\r\n",
        b"\r",
        b" ",
        b"\0",
        b"<<<<<<<<<<<< wide",
    ];
    let pick = |draw: u64, out_of: usize| -> usize {
        let out_of = u64::try_from(out_of).expect("a small count");
        usize::try_from(draw % out_of).expect("below a small count")
    };
    let started = std::time::Instant::now();
    for _ in 0..1_000 {
        let mut soup = Vec::new();
        let pieces_in_this = pick(next(), 40);
        for _ in 0..pieces_in_this {
            soup.extend_from_slice(pieces[pick(next(), pieces.len())]);
        }
        if let Ok(file) = parse(&soup) {
            assert_eq!(file.render(), soup, "{soup:?} does not round-trip");
            let _ = file.blocks();
        }
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "1 000 soups took {:?}",
        started.elapsed()
    );
}

#[test]
fn first_marker_line_agrees() {
    let dir = borrowed(Shape::StoppedRebase);
    for name in MARKER_FILES {
        let path = dir.join(name);
        let file = parse(&bytes(&path)).expect("parses");
        let from_scan = first_marker_line(&path).expect("scans");
        assert_eq!(from_scan, Some(file.blocks()[0].first_line), "{name}");
    }
}

#[test]
fn marker_size_other_than_seven_is_plain() {
    let twelve = b"a\n<<<<<<<<<<<< HEAD\nours\n============\ntheirs\n>>>>>>>>>>>> feature\n";
    let file = parse(twelve).expect("parses, as plain text");
    assert_eq!(file.open(), 0);
    assert_eq!(file.regions.len(), 1);
    assert_eq!(file.render(), twelve);
    let six = b"<<<<<< HEAD\nours\n======\ntheirs\n>>>>>> feature\n";
    assert_eq!(parse(six).expect("plain").open(), 0);
}

#[test]
fn malformed_files_name_their_line() {
    let cases: &[(&[u8], ParseError)] = &[
        (
            b"a\n=======\n",
            ParseError {
                line: 2,
                what: ParseErrorKind::StrayMarker,
            },
        ),
        (
            b">>>>>>> x\n",
            ParseError {
                line: 1,
                what: ParseErrorKind::StrayMarker,
            },
        ),
        (
            b"||||||| x\n",
            ParseError {
                line: 1,
                what: ParseErrorKind::StrayMarker,
            },
        ),
        (
            b"<<<<<<< a\nx\n<<<<<<< a\n=======\n>>>>>>> b\n",
            ParseError {
                line: 3,
                what: ParseErrorKind::StrayMarker,
            },
        ),
        (
            b"<<<<<<< a\nx\n>>>>>>> b\n",
            ParseError {
                line: 3,
                what: ParseErrorKind::StrayMarker,
            },
        ),
        (
            b"plain\n<<<<<<< a\nx\n=======\ny\n",
            ParseError {
                line: 2,
                what: ParseErrorKind::Unterminated,
            },
        ),
        (
            b"<<<<<<< a",
            ParseError {
                line: 1,
                what: ParseErrorKind::Unterminated,
            },
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(parse(input).unwrap_err(), *expected, "{input:?}");
    }
}

#[test]
fn a_second_base_marker_or_separator_inside_a_block_is_content() {
    let input = b"<<<<<<< a\n||||||| b\nbase\n||||||| again\n=======\ntheirs\n||||||| c\n=======\n>>>>>>> d\n";
    let file = parse(input).expect("parses");
    let block = one_block(&file);
    assert_eq!(
        block.base.as_ref().map(|(_, lines)| lines.clone()),
        Some(lines(&[b"base\n", b"||||||| again\n"]))
    );
    assert_eq!(
        block.theirs,
        lines(&[b"theirs\n", b"||||||| c\n", b"=======\n"])
    );
    assert_eq!(file.render(), input);
}

#[test]
fn blocks_report_index_state_and_line() {
    let input = b"one\ntwo\n<<<<<<< a\nx\n=======\ny\n>>>>>>> b\nthree\n<<<<<<< a\nx\n||||||| o\nz\n=======\ny\n>>>>>>> b\n";
    let mut file = parse(input).expect("parses");
    let before = file.blocks();
    assert_eq!(before.len(), 2);
    assert_eq!((before[0].index, before[0].first_line), (0, 3));
    assert_eq!(before[0].state, BlockState::Open);
    assert_eq!((before[1].index, before[1].first_line), (1, 9));
    assert_eq!(file.open(), 2);

    file.take(0, Take::Both).expect("block 0");
    let after = file.blocks();
    // Block 0 now renders two lines (x, y) where it rendered five.
    assert_eq!(after[0].first_line, 3);
    assert_eq!(after[0].state, BlockState::Took(Take::Both));
    assert_eq!(after[1].first_line, 6);
    assert_eq!(file.open(), 1);
    assert_eq!(
        file.render(),
        &b"one\ntwo\nx\ny\nthree\n<<<<<<< a\nx\n||||||| o\nz\n=======\ny\n>>>>>>> b\n"[..]
    );
    assert_eq!(file.take(2, Take::Ours), Err(super::NoSuchBlock(2)));
}

#[test]
fn a_hand_built_block_renders_with_its_ends() {
    let file = ConflictFile {
        regions: vec![Region::Conflict(Block {
            ours_label: b"HEAD".to_vec(),
            ours: lines(&[b"o\n"]),
            base: None,
            theirs_label: b"feature".to_vec(),
            theirs: lines(&[b"t\n"]),
            ends: MarkerEnds::LF,
        })],
        fingerprint: blob_id(b""),
    };
    assert_eq!(
        file.render(),
        &b"<<<<<<< HEAD\no\n=======\nt\n>>>>>>> feature\n"[..]
    );
}
