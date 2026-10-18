//! Whole-file cards: `o`/`t` per shape, `b` refused, and how much of the file a card reads (m6-conflicts/05 scope 1; m6-05 review, Major 4).

use super::*;

/// The whole-file card's own key text, per shape and per binary
/// (m6-conflicts/05 scope 1).
#[test]
fn whole_file_table_through_the_keys() {
    // Role words alone: the names a side has with no branch to name.
    let names = mergewright_ui::conflict_resolver::SideNames {
        ours: mergewright_term::sanitize_line("yours"),
        theirs: mergewright_term::sanitize_line("theirs"),
        roles: mergewright_ui::conflict_resolver::SideRoles::Merge,
    };
    for (how, binary, want) in [
        (
            UnmergedHow::DeletedByThem,
            false,
            "o keeps the file  t deletes it",
        ),
        (
            UnmergedHow::DeletedByUs,
            false,
            "o deletes it  t keeps theirs",
        ),
        (UnmergedHow::AddedByUs, false, "o keeps yours  t drops it"),
        (
            UnmergedHow::AddedByThem,
            false,
            "o drops it  t keeps theirs",
        ),
        (UnmergedHow::BothDeleted, false, "o or t removes it"),
        (
            UnmergedHow::BothModified,
            true,
            "o keeps yours  t keeps theirs",
        ),
        (
            UnmergedHow::BothAdded,
            true,
            "o keeps yours  t keeps theirs",
        ),
    ] {
        assert_eq!(
            mergewright::update::resolver::whole_file_keys(
                how,
                binary,
                &names,
                mergewright_ui::keymap::Keymap::shipped(),
            ),
            want,
            "{how:?} binary={binary}"
        );
    }
}

/// Every whole-file `(how, side)` row, driven through the real keys on the
/// real fixture: the exact `Operate` command `o`/`t` queue, and what the
/// file is once git has it (m6-conflicts/05 scope 1). `pic.bin` is the
/// binary `UU` row.
#[test]
#[allow(clippy::too_many_lines, reason = "a table of 12 (how, side) rows")]
fn whole_file_ops_per_row() {
    // (path, how, side, the file after: None = gone, Some(bytes) = kept)
    let rows: [(&str, UnmergedHow, Side, Option<&[u8]>); 12] = [
        (
            "theirs-deleted.txt",
            UnmergedHow::DeletedByThem,
            Side::Ours,
            Some(b"changed by us, deleted by them\n"),
        ),
        (
            "theirs-deleted.txt",
            UnmergedHow::DeletedByThem,
            Side::Theirs,
            None,
        ),
        (
            "ours-deleted.txt",
            UnmergedHow::DeletedByUs,
            Side::Ours,
            None,
        ),
        (
            "ours-deleted.txt",
            UnmergedHow::DeletedByUs,
            Side::Theirs,
            Some(b"deleted by us, changed by them\n"),
        ),
        (
            "added-by-us.txt",
            UnmergedHow::AddedByUs,
            Side::Ours,
            Some(b"renamed to a different name by each side\n"),
        ),
        (
            "added-by-us.txt",
            UnmergedHow::AddedByUs,
            Side::Theirs,
            None,
        ),
        (
            "added-by-them.txt",
            UnmergedHow::AddedByThem,
            Side::Ours,
            None,
        ),
        (
            "added-by-them.txt",
            UnmergedHow::AddedByThem,
            Side::Theirs,
            Some(b"renamed to a different name by each side\n"),
        ),
        (
            "both-deleted.txt",
            UnmergedHow::BothDeleted,
            Side::Ours,
            None,
        ),
        (
            "both-deleted.txt",
            UnmergedHow::BothDeleted,
            Side::Theirs,
            None,
        ),
        (
            "pic.bin",
            UnmergedHow::BothModified,
            Side::Ours,
            Some(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR ours\0"),
        ),
        (
            "pic.bin",
            UnmergedHow::BothModified,
            Side::Theirs,
            Some(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR theirs\0"),
        ),
    ];
    for (name, how, side, after) in rows {
        let (scratch, app, state) = stopped(Shape::StoppedMerge);
        let repo = scratch.path().to_owned();
        let state = resolver_on_whole_file(&app, state, name);
        let key = if side == Side::Ours { 'o' } else { 't' };

        let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char(key)));
        let [Command::Operate(_, operation, ..)] = commands.as_slice() else {
            panic!("{name} `{key}` queued {commands:?}")
        };
        assert!(
            matches!(
                operation.as_ref(),
                Operation::ResolveWholeFile(ResolveWholeParams { path, side: s, how: h })
                    if path == Path::new(name) && *s == side && *h == how
            ),
            "{name} `{key}`: {operation:?}"
        );

        let state = app.key(&state, KeyCode::Char(key));
        let state = app.pump(state, "the whole-file resolve", |state| {
            state.operating.is_none()
        });
        assert!(
            !still_unmerged(&repo).lines().any(|line| line == name),
            "{name} `{key}`: still unmerged"
        );
        match after {
            None => assert!(
                !repo.join(name).exists(),
                "{name} `{key}`: file still there"
            ),
            Some(bytes) => {
                let got = std::fs::read(repo.join(name)).expect("the kept file");
                assert_eq!(got, bytes, "{name} `{key}`: wrong side kept");
            }
        }

        drop(state);
        app.close();
        drop(scratch);
    }
}

/// `theirs-deleted.txt` is `UD` (`DeletedByThem`): a whole-file card, not a
/// closed resolver — the shape scope 1 turns from a refusal into a card.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one acceptance box walked end to end, in its own order"
)]
fn deleted_by_them_o_keeps() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");
    {
        let whole = open(&state)
            .whole_file
            .as_ref()
            .expect("a whole-file card, not a closed resolver");
        assert_eq!(whole.how, UnmergedHow::DeletedByThem);
        assert!(!whole.binary);
        assert_eq!(
            whole.on_disk.as_ref().map(|on_disk| on_disk.head.to_vec()),
            Some(vec![b"changed by us, deleted by them".to_vec()]),
            "the card shows what is on disk"
        );
    }
    let rows = crate::captures::frame(&state, 120, 40);
    assert!(
        rows.iter()
            .any(|row| row.contains("deleted by them · o keeps the file  t deletes it")),
        "the card's line is not on the frame:\n{}",
        rows.join("\n")
    );

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('o')));
    let [Command::Operate(_, operation, ..)] = commands.as_slice() else {
        panic!("`o` on the card queued {commands:?}")
    };
    assert!(
        matches!(
            operation.as_ref(),
            Operation::ResolveWholeFile(ResolveWholeParams {
                path,
                side: Side::Ours,
                how: UnmergedHow::DeletedByThem,
            }) if path == Path::new("theirs-deleted.txt")
        ),
        "{operation:?}"
    );

    let state = app.key(&state, KeyCode::Char('o'));
    assert!(state.operating.is_some(), "{:?}", state.status);
    let state = app.pump(state, "the resolve and the next file", |state| {
        state.operating.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path != Path::new("theirs-deleted.txt")
                    && open.job.is_none()
                    && (open.file.is_some() || open.whole_file.is_some())
            })
    });

    assert_eq!(
        on_disk(&repo, "theirs-deleted.txt"),
        "changed by us, deleted by them\n",
        "`o` kept our side, staged"
    );
    assert!(
        !still_unmerged(&repo)
            .lines()
            .any(|line| line == "theirs-deleted.txt"),
        "git no longer calls it unmerged: {:?}",
        still_unmerged(&repo)
    );
    assert!(
        still_unmerged(&repo)
            .lines()
            .any(|line| Path::new(line) == open(&state).path),
        "the resolver moved on to another conflicted file: {:?}",
        open(&state).path
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn deleted_by_them_t_deletes() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");

    let state = app.key(&state, KeyCode::Char('t'));
    assert!(
        state.operating.is_some(),
        "`t` on a whole-file card queued no write: {:?}",
        state.status
    );
    let state = app.pump(state, "the whole-file resolve", |state| {
        state.operating.is_none()
    });

    assert!(
        !repo.join("theirs-deleted.txt").exists(),
        "`t` removed the file theirs never had"
    );
    assert_eq!(
        crate::app_harness::git(&repo, &["ls-files", "--", "theirs-deleted.txt"]).trim(),
        "",
        "and the index holds no entry for it at any stage"
    );
    assert!(
        !still_unmerged(&repo)
            .lines()
            .any(|line| line == "theirs-deleted.txt"),
        "git no longer calls it unmerged: {:?}",
        still_unmerged(&repo)
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// `b` has no meaning on a whole-file card: there are no blocks to keep both
/// sides of.
#[test]
fn b_refused_on_whole_file() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");

    let state = app.key(&state, KeyCode::Char('b'));
    assert!(state.operating.is_none(), "`b` wrote nothing");
    let Status::Sticky { reason, keys } = &state.status else {
        panic!("`b` on a whole-file card said {:?}", state.status)
    };
    assert_eq!(reason, "no both for a deleted by them file");
    assert_eq!(keys.as_deref(), Some("o yours  t theirs"));
    assert!(open(&state).whole_file.is_some(), "the card is still up");

    drop(state);
    app.close();
    drop(scratch);
}

/// A large file under a whole-file card is not read whole (m6-05 review,
/// Major 4): the card holds its first lines only, and says how much more
/// there is.
#[test]
fn a_large_card_reads_only_its_head() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let big: String = (0..100_000).fold(String::new(), |mut big, n| {
        big.push_str("line ");
        big.push_str(&n.to_string());
        big.push('\n');
        big
    });
    std::fs::write(scratch.path().join("theirs-deleted.txt"), &big).expect("writable");
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");
    let whole = open(&state).whole_file.as_ref().expect("a card");
    let on_disk = whole.on_disk.as_ref().expect("the file is there");
    assert_eq!(on_disk.head.len(), 500, "`jobs::conflict::CARD_LINES`");
    assert!(on_disk.cut);
    assert_eq!(on_disk.size, u64::try_from(big.len()).expect("fits"));
    let rows = crate::captures::frame(&state, 120, 40);
    assert!(
        rows.iter().any(|row| row.contains("line 0")),
        "the head is drawn:\n{}",
        rows.join("\n")
    );
    // The window shows the top; the last line says how much was left.
    let state = app.key(&state, KeyCode::Char('G'));
    let rows = crate::captures::frame(&state, 120, 40);
    assert!(
        rows.iter()
            .any(|row| row.contains("… first 500 lines of 1.0 MB · e opens the whole file")),
        "the cut is said:\n{}",
        rows.join("\n")
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// A binary card reads none of the file's bytes: it shows the size alone,
/// never the bytes as lossy text "lines" (m6-05 review, Major 4).
#[test]
fn a_binary_card_shows_its_size_not_its_bytes() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let size = std::fs::metadata(scratch.path().join("pic.bin"))
        .expect("the binary is on disk")
        .len();
    let state = resolver_on_whole_file(&app, state, "pic.bin");
    let whole = open(&state).whole_file.as_ref().expect("a card");
    assert!(whole.binary);
    let on_disk = whole.on_disk.as_ref().expect("the file is there");
    assert!(on_disk.head.is_empty(), "no bytes held");
    let rows = crate::captures::frame(&state, 120, 40);
    assert!(size < 1024, "the fixture's binary is small: {size}");
    let expected = format!("binary · {size} B");
    assert!(
        rows.iter().any(|row| row.contains(&expected)),
        "{expected:?} is not on the frame:\n{}",
        rows.join("\n")
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// Review-28 m2 (ticket 30): the app's answer to `d` on a binary `UU` card
/// is ops' answer, not a rule of its own. `pic.bin` is the fixture's binary
/// `UU`: `d` opens the box exactly when ops' `restart_gate` admits it.
#[test]
fn a_binary_uu_card_answers_d_by_ops_rule() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let handle = mergewright_git::repo::RepoHandle::open(scratch.path()).expect("the repo opens");
    let git = mergewright_git::version::system_git()
        .expect("git")
        .version();
    let facts = mergewright_ops::facts::read_facts(
        &handle,
        git,
        &(),
        &mergewright_ops::operation::FactsScope::default(),
    )
    .expect("facts read");
    let ops_admits = mergewright_ops::apply::restart_gate(&handle, &facts, Path::new("pic.bin"))
        .expect("the gate answers")
        .is_ok();

    let state = resolver_on_whole_file(&app, state, "pic.bin");
    assert!(open(&state).whole_file.as_ref().is_some_and(|w| w.binary));
    let state = app.key(&state, KeyCode::Char('d'));
    assert_eq!(
        state.confirm.is_some(),
        ops_admits,
        "`d` on a binary UU: the box opens exactly when ops admits the restart"
    );
    drop(state);
    app.close();
    drop(scratch);
}
