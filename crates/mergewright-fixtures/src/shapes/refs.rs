//! Refs, tags and the states `HEAD` can be in — the half of a repository
//! that is not commits. Most of these need plain `git` after the stream,
//! because a `fast-import` stream cannot detach `HEAD`, stash, or leave a
//! branch unborn.

use super::{BuildError, MAIN, REMOTE_URL, Site, file};
use crate::stream::Stream;

pub(super) fn tags(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let first = stream.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Tagged.\n")],
    );
    let second = stream.commit(MAIN, b"Add a.txt\n", &[first], &[file(b"a.txt", b"1\n")]);
    let third = stream.commit(MAIN, b"Add b.txt\n", &[second], &[file(b"b.txt", b"1\n")]);
    let fourth = stream.commit(MAIN, b"Add c.txt\n", &[third], &[file(b"c.txt", b"1\n")]);
    stream.point(b"refs/tags/v0.1", first);
    stream.point(b"refs/tags/v0.2", second);
    stream.annotated_tag(
        b"v1.0",
        third,
        b"Release 1.0\n\nThe first tagged release.\n",
    );
    stream.annotated_tag(b"v2.0", fourth, b"Release 2.0\n");
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn unicode_names(site: &Site<'_>) -> Result<(), BuildError> {
    const AUTHOR: &str = "Ünïcödé Fixture <unicode@example.invalid>";
    let mut stream = Stream::new();
    let root = stream.commit_by(
        AUTHOR,
        MAIN,
        "Ajoute le fichier café ☕\n".as_bytes(),
        &[],
        &[file("café/naïve.txt".as_bytes(), "contenu\n".as_bytes())],
    );
    let second = stream.commit_by(
        AUTHOR,
        MAIN,
        "日本語のコミットメッセージ\n".as_bytes(),
        &[root],
        &[file("日本語.txt".as_bytes(), "内容\n".as_bytes())],
    );
    stream.commit_by(
        AUTHOR,
        "refs/heads/fonctionnalité".as_bytes(),
        "fonctionnalité : première étape 🚀\n".as_bytes(),
        &[second],
        &[file("café/étape.txt".as_bytes(), "étape\n".as_bytes())],
    );
    stream.commit_by(
        AUTHOR,
        "refs/heads/機能".as_bytes(),
        "機能: 最初のコミット\n".as_bytes(),
        &[second],
        &[file("機能.txt".as_bytes(), "内容\n".as_bytes())],
    );
    stream.commit_by(
        AUTHOR,
        "refs/heads/ветка".as_bytes(),
        "ветка: первый коммит\n".as_bytes(),
        &[second],
        &[file("документ.txt".as_bytes(), "содержимое\n".as_bytes())],
    );
    stream.annotated_tag("версия-1.0".as_bytes(), second, "Релиз 1.0 🎉\n".as_bytes());
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn ref_states(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let base = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let main_tip = stream.commit(
        MAIN,
        b"Advance main\n",
        &[base],
        &[file(b"main.txt", b"1\n")],
    );
    let pushed = stream.commit(
        MAIN,
        b"Advance main again\n",
        &[main_tip],
        &[file(b"main.txt", b"2\n")],
    );
    // Level with its upstream.
    stream.point(b"refs/remotes/origin/main", pushed);

    // One commit ahead of its upstream.
    let ahead_1 = stream.commit(
        b"refs/heads/ahead",
        b"ahead: pushed\n",
        &[main_tip],
        &[file(b"ahead.txt", b"1\n")],
    );
    stream.commit(
        b"refs/heads/ahead",
        b"ahead: not pushed yet\n",
        &[ahead_1],
        &[file(b"ahead.txt", b"2\n")],
    );
    stream.point(b"refs/remotes/origin/ahead", ahead_1);

    // One commit behind its upstream.
    let behind_1 = stream.commit(
        b"refs/heads/behind",
        b"behind: shared\n",
        &[main_tip],
        &[file(b"behind.txt", b"1\n")],
    );
    stream.commit(
        b"refs/remotes/origin/behind",
        b"behind: theirs\n",
        &[behind_1],
        &[file(b"behind.txt", b"2\n")],
    );

    // A commit no branch reaches: the temporary branch below is deleted, and
    // only the tag is left pointing at it.
    let archived = stream.commit(
        b"refs/heads/to-archive",
        b"Archived work\n",
        &[base],
        &[file(b"archived.txt", b"a\n")],
    );
    stream.point(b"refs/tags/archived", archived);

    site.import(stream)?;
    site.git(&["update-ref", "-d", "refs/heads/to-archive"])?;
    site.git(&["remote", "add", "origin", REMOTE_URL])?;
    for branch in ["main", "ahead", "behind"] {
        site.git(&["config", &format!("branch.{branch}.remote"), "origin"])?;
        site.git(&[
            "config",
            &format!("branch.{branch}.merge"),
            &format!("refs/heads/{branch}"),
        ])?;
    }
    site.checkout_main()
}

pub(super) fn detached_head(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let mut tip = stream.commit(MAIN, b"Commit 1\n", &[], &[file(b"log.txt", b"1\n")]);
    for n in 2..=4u32 {
        let message = format!("Commit {n}\n");
        let content = format!("{n}\n");
        tip = stream.commit(
            MAIN,
            message.as_bytes(),
            &[tip],
            &[file(b"log.txt", content.as_bytes())],
        );
    }
    site.import(stream)?;
    site.checkout_main()?;
    site.git(&["checkout", "-q", "--detach", "refs/heads/main~2"])?;
    Ok(())
}

pub(super) fn unborn_branch(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let root = stream.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Unborn.\n")],
    );
    stream.commit(MAIN, b"Add a.txt\n", &[root], &[file(b"a.txt", b"1\n")]);
    stream.commit(
        b"refs/heads/other",
        b"other: work\n",
        &[root],
        &[file(b"other.txt", b"o\n")],
    );
    site.import(stream)?;
    site.checkout_main()?;
    // `HEAD` now names a branch that has no commit — the state a fresh
    // `git init` is in, but with other branches already present.
    site.git(&["checkout", "-q", "--orphan", "fresh"])?;
    site.git(&["rm", "-r", "-f", "-q", "."])?;
    Ok(())
}

pub(super) fn stash(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let root = stream.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Stashes.\n")],
    );
    stream.commit(
        MAIN,
        b"Add a.txt\n",
        &[root],
        &[file(b"a.txt", b"committed\n")],
    );
    site.import(stream)?;
    site.checkout_main()?;

    site.write("a.txt", "work in progress\n")?;
    site.git(&["stash", "push", "-q", "-m", "wip: adjust a.txt"])?;

    site.write("a.txt", "second attempt\n")?;
    site.write("untracked.txt", "not added yet\n")?;
    site.git(&[
        "stash",
        "push",
        "-q",
        "-u",
        "-m",
        "wip: with an untracked file",
    ])?;
    Ok(())
}

pub(super) fn non_utf8_path(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let root = stream.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Latin-1 paths.\n")],
    );
    // `\xe9` is `é` in Latin-1 and not valid UTF-8 in any encoding git cares
    // about. A path lives inside a tree object, so every filesystem can hold
    // this — which is why the working tree is left on the readme commit and
    // the Latin-1 path is never checked out.
    stream.commit(
        b"refs/heads/latin1",
        b"Add a Latin-1 path\n",
        &[root],
        &[file(b"caf\xe9.txt", b"latin-1 path\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn non_utf8_ref_name(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let root = stream.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Latin-1 ref names.\n")],
    );
    // A loose ref is a file named after the branch, so this fixture exists
    // only where the filesystem accepts a name that is not valid UTF-8.
    stream.commit(
        b"refs/heads/caf\xe9",
        b"Work under a Latin-1 branch name\n",
        &[root],
        &[file(b"work.txt", b"under a latin-1 branch\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}
