//! The parse rules, checked against files git itself wrote (the stopped
//! corpus shapes) and against hand-written edges git never writes.

use std::path::{Path, PathBuf};

use mergewright_fixtures::Shape;

use super::{
    BaseMismatch, Block, BlockState, ConflictFile, Line, MarkerEnds, ParseError, ParseErrorKind,
    Projection, ReadError, Region, Take, WriteError, attach_base, blob_id, diff3_file, parse, read,
    stage_bytes, write_back,
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
    mergewright_fixtures::borrow(shape)
}

fn open(dir: &Path) -> RepoHandle {
    RepoHandle::open(dir).expect("the fixture opens")
}

fn bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|why| panic!("reading {}: {why}", path.display()))
}

fn git(dir: &Path, args: &[&str]) -> Vec<u8> {
    mergewright_fixtures::git::run(dir, args).unwrap_or_else(|why| panic!("{why}"))
}

fn lines(text: &[&[u8]]) -> Vec<Line> {
    text.iter().map(|line| Line(line.to_vec())).collect()
}

/// `first_marker_line` over bytes: written to a scratch file and scanned.
fn scan(input: &[u8]) -> Option<usize> {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = dir.path().join("f.txt");
    std::fs::write(&path, input).expect("writes");
    first_marker_line(&path).expect("scans")
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
    assert_eq!(block.ours_label.as_deref(), Some(&b"HEAD"[..]));
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
            let base = block
                .base
                .as_ref()
                .expect("a zdiff3 block carries its base");
            assert_eq!(base.label.as_deref(), Some(&b"base"[..]));
            assert_eq!(base.lines, lines(&[b"middle\n"]));
        }
    }
    let plain_style = parse(&bytes(&dir.join("both.txt"))).expect("both.txt parses");
    assert!(one_block(&plain_style).base.is_none());
}

#[test]
fn take_matches_git_checkout_sides() {
    let scratch = mergewright_fixtures::copy_into_temp(Shape::StoppedMerge);
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

#[test]
fn take_all_equals_taking_each_block() {
    let dir = borrowed(Shape::StoppedMerge);
    for name in MARKER_FILES {
        let input = bytes(&dir.join(name));
        for take in [Take::Ours, Take::Theirs, Take::Both] {
            let parsed = parse(&input).unwrap_or_else(|why| panic!("{name}: {why}"));
            let mut each = parsed.clone();
            let open: Vec<usize> = each
                .blocks()
                .iter()
                .filter(|at| at.state == BlockState::Open)
                .map(|at| at.index)
                .collect();
            for index in &open {
                each.take(*index, take).expect("the block exists");
            }
            let mut all = parsed;
            assert_eq!(all.take_all(take), open.len(), "{name} {take:?}");
            assert_eq!(all.render(), each.render(), "{name} {take:?}");
        }
    }
}

#[test]
fn take_all_keeps_blocks_already_taken() {
    let dir = borrowed(Shape::StoppedMerge);
    let mut file = parse(&bytes(&dir.join("three.txt"))).expect("three.txt parses");
    file.take(0, Take::Ours).expect("block 0 exists");
    assert_eq!(file.take_all(Take::Theirs), 2);
    let blocks = file.blocks();
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].state, BlockState::Took(Take::Ours));
    assert_eq!(blocks[1].state, BlockState::Took(Take::Theirs));
    assert_eq!(blocks[2].state, BlockState::Took(Take::Theirs));
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
    let scratch = mergewright_fixtures::copy_into_temp(Shape::StoppedRebase);
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

    let scratch = mergewright_fixtures::copy_into_temp(Shape::StoppedRebase);
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
        .filter(|name| name.starts_with(".mwr-"))
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
            mergewright_fixtures::git::run_isolated(&dir, &["hash-object", "--stdin"], Some(input))
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

/// The reproduced base lines of every block that was given one, in order.
fn attached_bases(file: &ConflictFile) -> Vec<Vec<Line>> {
    file.regions
        .iter()
        .filter_map(|region| match region {
            Region::Conflict(block) | Region::Resolved { block, .. } => block
                .reproduced
                .as_ref()
                .map(|reproduced| reproduced.base.lines.clone()),
            Region::Plain(_) => None,
        })
        .collect()
}

/// Every block's `shared` flag, for the blocks that got a reproduced base.
fn attached_shared(file: &ConflictFile) -> Vec<bool> {
    file.regions
        .iter()
        .filter_map(|region| match region {
            Region::Conflict(block) | Region::Resolved { block, .. } => block
                .reproduced
                .as_ref()
                .map(|reproduced| reproduced.shared),
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
    // One file block, one run: the base is this block's own, not shared.
    assert_eq!(attached_shared(&file), vec![false]);
    let Region::Conflict(block) = &file.regions[1] else {
        panic!("region 1 is the block: {:?}", file.regions);
    };
    let reproduced = block.reproduced.as_ref().expect("a base was attached");
    assert_eq!(reproduced.base.label.as_deref(), Some(&b"base"[..]));
    // The file on disk had no `|||||||`, and still has none.
    assert!(block.base.is_none());
    assert_eq!(file.render(), MERGE_STYLE_JOINED);

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

/// The other direction, and the one the first round got wrong: git's zealous
/// refinement **splits** a conflict whose sides share interior lines, so two
/// file blocks are one reproduced block. Both literals are what git 2.54
/// printed on 2026-09-22 for base `1…7`, ours `A b c d e f B`, theirs `X b c
/// d e f Y` — the left one from a real `git merge` into the working tree, the
/// right one from `git merge-file -p --diff3` on its three stages.
const MERGE_STYLE_SPLIT: &[u8] = b"<<<<<<< HEAD\nA\n=======\nX\n>>>>>>> feature\nb\nc\nd\ne\nf\n<<<<<<< HEAD\nB\n=======\nY\n>>>>>>> feature\n";
const DIFF3_STYLE_ONE_BLOCK: &[u8] = b"<<<<<<< ours\nA\nb\nc\nd\ne\nf\nB\n||||||| base\n1\n2\n3\n4\n5\n6\n7\n=======\nX\nb\nc\nd\ne\nf\nY\n>>>>>>> theirs\n";

#[test]
fn a_block_git_split_gets_the_whole_regions_base_marked_shared() {
    let diff3 = parse(DIFF3_STYLE_ONE_BLOCK).expect("git's diff3 output parses");
    assert_eq!(diff3.open(), 1);

    let mut file = parse(MERGE_STYLE_SPLIT).expect("git's merge output parses");
    assert_eq!(file.open(), 2);
    let before = file.blocks();
    assert_eq!(attach_base(&mut file, &diff3), Ok(2));

    // Both blocks get the whole region's base, and both say it is shared:
    // git cannot attribute those lines to one half or the other.
    let whole = lines(&[b"1\n", b"2\n", b"3\n", b"4\n", b"5\n", b"6\n", b"7\n"]);
    assert_eq!(attached_bases(&file), vec![whole.clone(), whole]);
    assert_eq!(attached_shared(&file), vec![true, true]);

    // M4: attaching changed neither the bytes nor the line numbers.
    assert_eq!(file.render(), MERGE_STYLE_SPLIT);
    assert_eq!(file.blocks(), before);

    // A hand edit in the split region is still a mismatch, with a message
    // that does not blame edits on a file nobody edited.
    let mut edited = parse(MERGE_STYLE_SPLIT).expect("parses");
    hand_edit(&mut edited, 1);
    let why = attach_base(&mut edited, &diff3).expect_err("no run matches");
    assert_eq!(why, BaseMismatch { block: 0 });
    let said = why.to_string();
    assert!(!said.contains("edit"), "{said}");
    assert!(said.contains("no base"), "{said}");
}

#[test]
fn base_attaches_by_equality_only() {
    let scratch = mergewright_fixtures::copy_into_temp(Shape::StoppedRebase);
    let dir = scratch.path();
    let repo = open(dir);
    let path = dir.join("three.txt");
    let diff3 = three_txt_diff3(&repo);

    let mut file = read(&path).expect("three.txt reads");
    let before_blocks = file.blocks();
    let before_bytes = file.render();
    assert_eq!(attach_base(&mut file, &diff3), Ok(3));
    assert_eq!(
        attached_bases(&file),
        vec![lines(&[b"one\n"]), lines(&[b"two\n"]), lines(&[b"three\n"])]
    );
    assert_eq!(attached_shared(&file), vec![false, false, false]);
    let Region::Conflict(middle) = &file.regions[3] else {
        panic!("region 3 is the middle block: {:?}", file.regions);
    };
    assert_eq!(middle.ours, lines(&[b"two-ours\n"]));
    assert_eq!(middle.theirs, lines(&[b"two-theirs\n"]));

    // M4: a reproduced base is not part of the file. `write_back` would put
    // the same bytes back, and every block still starts on the line it does
    // on disk — what `$EDITOR` and the diff pane show.
    assert_eq!(file.render(), before_bytes);
    assert_eq!(file.render(), bytes(&path));
    assert_eq!(file.blocks(), before_blocks);

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
}

/// A repository whose `.gitattributes` makes one path CRLF in the working
/// tree while its blobs stay LF (`text eol=crlf`; `core.autocrlf=true` and an
/// `ident`/smudge filter do the same thing). Built here rather than added to
/// the shared corpus: it needs its own `.gitattributes`, and the corpus'
/// unmerged paths are counted by other tests.
///
/// Measured on git 2.54, 2026-09-22: the stop writes the conflict file CRLF
/// throughout, markers included, while `git show :2:f.txt` is LF.
fn crlf_attribute_stop() -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let dir = scratch.path();
    mergewright_fixtures::git::init_repo(dir).expect("git init");
    std::fs::write(dir.join(".gitattributes"), b"f.txt text eol=crlf\n").expect("attributes");
    let commit = |name: &str, content: &[u8]| {
        std::fs::write(dir.join("f.txt"), content).expect("f.txt");
        git(dir, &["add", "-A", "--", "."]);
        git(dir, &["commit", "-q", "-m", name]);
    };
    commit("base", b"one\ntwo\nthree\n");
    git(dir, &["checkout", "-q", "-b", "feature"]);
    commit("theirs", b"one\ntwo-theirs\nthree\n");
    git(dir, &["checkout", "-q", "main"]);
    commit("ours", b"one\ntwo-ours\nthree\n");
    // The merge is meant to stop on conflicts (exit 1); anything else is a
    // broken recipe, so it is not run through `git` (which insists on 0).
    let stopped = mergewright_fixtures::git::run(dir, &["merge", "--no-edit", "feature"]);
    match stopped {
        Err(mergewright_fixtures::git::GitError::Failed { status, .. })
            if status.code() == Some(1) => {}
        other => panic!("`git merge` should have stopped on a conflict: {other:?}"),
    }
    scratch
}

#[test]
fn a_crlf_checkout_still_gets_its_base() {
    let scratch = crlf_attribute_stop();
    let dir = scratch.path();
    let repo = open(dir);
    let path = dir.join("f.txt");

    // The premise, asserted rather than assumed: CRLF on disk, LF in the
    // index. Comparing the raw stages against this file matches nothing.
    let on_disk = bytes(&path);
    assert!(
        on_disk.starts_with(b"one\r\n<<<<<<< HEAD\r\n"),
        "{on_disk:?}"
    );
    let raw = stage_bytes(&repo, Path::new("f.txt"), 2)
        .expect("stage 2 reads")
        .expect("stage 2 is there");
    assert_eq!(raw, b"one\ntwo-ours\nthree\n", "stage_bytes stays raw");

    let diff3 = diff3_file(&repo, Path::new("f.txt"))
        .expect("merge-file runs")
        .expect("every stage is there");
    let mut file = read(&path).expect("f.txt reads");
    assert_eq!(file.open(), 1);
    assert_eq!(
        attach_base(&mut file, &diff3),
        Ok(1),
        "a pristine CRLF checkout has a base"
    );
    assert_eq!(attached_bases(&file), vec![lines(&[b"two\r\n"])]);
    assert_eq!(file.render(), on_disk);
}

#[test]
fn base_survives_a_written_take_but_not_a_hand_edit() {
    let scratch = mergewright_fixtures::copy_into_temp(Shape::StoppedRebase);
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
    for shape in Shape::STOPPED {
        let dir = borrowed(*shape);
        for name in MARKER_FILES {
            let path = dir.join(name);
            let file = parse(&bytes(&path)).expect("parses");
            let from_scan = first_marker_line(&path).expect("scans");
            assert_eq!(
                from_scan,
                Some(file.blocks()[0].first_line),
                "{name} in {shape:?}"
            );
        }
    }
}

/// What git 2.54 wrote into the working tree on 2026-09-22 for a path with
/// `.gitattributes` `f.txt conflict-marker-size=12`: a real `UU` file whose
/// markers this parser cannot read.
const MARKER_SIZE_TWELVE: &[u8] =
    b"one\n<<<<<<<<<<<< HEAD\ntwo-ours\n============\ntwo-theirs\n>>>>>>>>>>>> feature\nthree\n";

#[test]
fn a_marker_size_over_seven_is_refused_not_called_plain() {
    // Calling this plain is the work-loss path: no blocks, so the resolver
    // has nothing to open and nothing can refuse the stage.
    assert_eq!(
        parse(MARKER_SIZE_TWELVE).unwrap_err(),
        ParseError {
            line: 2,
            what: ParseErrorKind::MarkerSize,
        }
    );
    // And the scan that guards a stage does see it.
    assert!(
        ParseError {
            line: 2,
            what: ParseErrorKind::MarkerSize
        }
        .to_string()
        .contains("conflict-marker-size")
    );

    // Below seven is not caught here (`>>> ` at column 0 is a Python prompt,
    // `<<< ` is rare but the pair is not worth the false positives); it
    // parses as plain text, as it did before.
    let three = b"one\n<<< HEAD\ntwo-ours\n===\ntwo-theirs\n>>> feature\nthree\n";
    assert_eq!(parse(three).expect("plain").open(), 0);
    let six = b"<<<<<< HEAD\nours\n======\ntheirs\n>>>>>> feature\n";
    assert_eq!(parse(six).expect("plain").open(), 0);

    // A long run of `=` or `>` stays content: a Markdown setext underline is
    // exactly `========`, and is why a project sets the attribute at all.
    let setext = b"Introduction\n============\n\nbody\n";
    let file = parse(setext).expect("a plain Markdown file is not a marker file");
    assert_eq!(file.open(), 0);
    assert_eq!(file.render(), setext);
    let quoted = b"a\n>>>>>>>>> deeply quoted\n";
    assert_eq!(parse(quoted).expect("plain").render(), &quoted[..]);

    // Inside a block a long `<` run is content, not a size error: a file the
    // parser got inside of has seven-character markers.
    let inside = b"<<<<<<< HEAD\n<<<<<<<<<<<< not a marker\n=======\nt\n>>>>>>> feature\n";
    let file = parse(inside).expect("parses");
    assert_eq!(file.render(), &inside[..]);
    assert_eq!(
        one_block(&file).ours,
        lines(&[b"<<<<<<<<<<<< not a marker\n"])
    );
}

/// git's `rerere` preimage, byte for byte, from a stop with `rerere.enabled`
/// on git 2.54 (2026-09-22): bare markers, no space and no label. M7's
/// rerere reads these files.
const RERERE_PREIMAGE: &[u8] = b"one\n<<<<<<<\nb-o\n=======\nb-t\n>>>>>>>\nthree\n";

#[test]
fn bare_markers_are_markers_and_stay_byte_distinct() {
    let file = parse(RERERE_PREIMAGE).expect("a rerere preimage parses");
    assert_eq!(file.open(), 1);
    assert_eq!(file.render(), RERERE_PREIMAGE);
    let block = one_block(&file);
    assert_eq!(block.ours_label, None);
    assert_eq!(block.theirs_label, None);
    assert_eq!(block.ours, lines(&[b"b-o\n"]));
    assert_eq!(block.theirs, lines(&[b"b-t\n"]));
    assert_eq!(scan(RERERE_PREIMAGE), Some(2));
    // The oversize opener is seen by the scan too, though `parse` refuses
    // the file: the guard before a stage must never miss a real marker.
    assert_eq!(scan(MARKER_SIZE_TWELVE), Some(2));
}

#[test]
fn a_bare_label_and_an_empty_one_are_different_bytes() {
    // `None` and an empty label are different bytes and stay different.
    let empty = b"<<<<<<< \no\n||||||| \nb\n=======\nt\n>>>>>>> \n";
    let with_empty = parse(empty).expect("parses");
    assert_eq!(with_empty.render(), &empty[..]);
    let block = one_block(&with_empty);
    assert_eq!(block.ours_label.as_deref(), Some(&b""[..]));
    assert_eq!(
        block.base.as_ref().and_then(|base| base.label.as_deref()),
        Some(&b""[..])
    );

    // A bare base marker too, and the CRLF form of all of them.
    let bare_base = b"<<<<<<<\r\no\r\n|||||||\r\nb\r\n=======\r\nt\r\n>>>>>>>\r\n";
    let file = parse(bare_base).expect("parses");
    assert_eq!(file.render(), &bare_base[..]);
    let block = one_block(&file);
    assert_eq!(block.base.as_ref().expect("a base").label, None);

    // Bare markers out of place are stray, the same as labelled ones.
    for input in [&b"a\n>>>>>>>\n"[..], &b"|||||||\n"[..]] {
        assert_eq!(
            parse(input).unwrap_err().what,
            ParseErrorKind::StrayMarker,
            "{input:?}"
        );
    }
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
        block.base.as_ref().map(|base| base.lines.clone()),
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
            ours_label: Some(b"HEAD".to_vec()),
            ours: lines(&[b"o\n"]),
            base: None,
            theirs_label: Some(b"feature".to_vec()),
            theirs: lines(&[b"t\n"]),
            ends: MarkerEnds::LF,
            reproduced: None,
        })],
        fingerprint: blob_id(b""),
    };
    assert_eq!(
        file.render(),
        &b"<<<<<<< HEAD\no\n=======\nt\n>>>>>>> feature\n"[..]
    );
}

/// Projection `p`'s lines for each region, as a reader of that version
/// would see them.
fn projected(region: &Region, p: Projection) -> Vec<Line> {
    match (region, p) {
        (Region::Plain(lines), _) => lines.clone(),
        (Region::Conflict(block) | Region::Resolved { block, .. }, Projection::Ours)
        | (Region::Conflict(block), Projection::Result) => block.ours.clone(),
        (Region::Conflict(block) | Region::Resolved { block, .. }, Projection::Theirs) => {
            block.theirs.clone()
        }
        (Region::Resolved { block, take }, Projection::Result) => {
            block.taken(*take).cloned().collect()
        }
    }
}

fn assert_contiguous(file: &ConflictFile, what: &str) {
    for p in [Projection::Ours, Projection::Theirs, Projection::Result] {
        let firsts = file.first_lines(p);
        assert_eq!(
            firsts.len(),
            file.regions.len(),
            "{what} {p:?}: one per region"
        );
        let mut numbered = Vec::new();
        for (region, first) in file.regions.iter().zip(&firsts) {
            let lines = projected(region, p);
            assert_eq!(*first, numbered.len() + 1, "{what} {p:?}: a gap or overlap");
            numbered.extend((0..lines.len()).map(|i| first + i));
        }
        assert!(
            numbered.iter().enumerate().all(|(i, n)| *n == i + 1),
            "{what} {p:?}: numbers are not 1..=n"
        );
    }
    // Every block taken: Result is the rendered file, line for line.
    for take in [Take::Ours, Take::Theirs, Take::Both] {
        let mut taken = file.clone();
        for index in 0..taken.blocks().len() {
            taken.take(index, take).expect("the block exists");
        }
        let rendered = taken.render();
        let result: Vec<u8> = taken
            .regions
            .iter()
            .flat_map(|region| projected(region, Projection::Result))
            .flat_map(|line| line.0)
            .collect();
        assert_eq!(result, rendered, "{what} {take:?}: Result is not render()");
        let firsts = taken.first_lines(Projection::Result);
        let mut blocks = taken.blocks().into_iter();
        for (region, first) in taken.regions.iter().zip(firsts) {
            if !matches!(region, Region::Plain(_)) {
                let block = blocks.next().expect("one BlockAt per block");
                assert_eq!(
                    block.first_line, first,
                    "{what} {take:?}: disagrees with blocks()"
                );
            }
        }
    }
}

#[test]
fn first_lines_number_each_projection_without_gaps() {
    for shape in [Shape::StoppedMerge, Shape::StoppedRebase] {
        let dir = borrowed(shape);
        let conflicted = git(&dir, &["diff", "--name-only", "--diff-filter=U"]);
        let mut checked = 0;
        for name in String::from_utf8_lossy(&conflicted).lines() {
            let path = dir.join(name);
            if !path.exists() {
                continue; // deleted on a side: a whole-file conflict, no text
            }
            let Ok(file) = parse(&bytes(&path)) else {
                continue; // binary or unparsable: the resolver never numbers it
            };
            if file.open() == 0 {
                continue; // a whole-file conflict: no markers to number
            }
            assert_contiguous(&file, &format!("{shape:?} {name}"));
            checked += 1;
        }
        assert!(
            checked > 0,
            "{shape:?} holds no conflicted file with markers"
        );
    }

    // A generated set: plain runs and blocks of varied sizes.
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move |out_of: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        usize::try_from(state % out_of).expect("small")
    };
    for round in 0..200 {
        let mut text = String::new();
        for _ in 0..next(5) {
            for i in 0..next(4) {
                text.push_str(&format!("plain {i}\n"));
            }
            text.push_str("<<<<<<< a\n");
            for i in 0..next(4) {
                text.push_str(&format!("ours {i}\n"));
            }
            if next(2) == 0 {
                text.push_str("||||||| b\nbase\n");
            }
            text.push_str("=======\n");
            for i in 0..next(4) {
                text.push_str(&format!("theirs {i}\n"));
            }
            text.push_str(">>>>>>> c\n");
        }
        let file = parse(text.as_bytes()).expect("generated files parse");
        assert_contiguous(&file, &format!("generated {round}"));
    }
}
