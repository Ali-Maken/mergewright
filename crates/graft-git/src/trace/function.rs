//! The function around a line, as git's funcname rules see it, and a
//! working-tree line's place in `HEAD`.
//!
//! **Method** (the ticket allows another than its sketch; this one is its
//! sketch, done for every line at once). The file as of the commit goes into
//! a temporary directory twice: as it is, and with a unique marker line
//! inserted after **every** line. `git diff --no-index -U0` between the two
//! gives one hunk per line, and each hunk's header is the funcname git finds
//! searching back from that line — the line itself included. So one diff
//! answers "which header governs line N" for every N; a function starts
//! where the header changes, and ends before the next header at the same or
//! shallower indentation (or at the file's end), trailing blank lines left
//! out.
//!
//! **Which driver.** The temporary files keep the path's base name, so the
//! repository's own basename patterns (`*.rs diff=rust`) and the user's
//! `core.attributesFile` still apply to them. When the repository's
//! attributes name no `diff` driver for the path and the user has no
//! `core.attributesFile`, Graft passes `-c core.attributesFile=<a file of
//! its own>` naming git's built-in driver for the extension — otherwise git's
//! default heuristic names the enclosing `impl`, not the method. When the
//! repository names a driver, the same file names that driver for `*`, so a
//! directory pattern (`src/**`) that cannot match a temporary path is still
//! honoured. A user `core.attributesFile` is never overridden.

use std::path::Path;

use graft_core::oid::Oid;

use super::LineRange;
use crate::blame::path_bytes;
use crate::diff::{Hunk, LineKind, parse_unified};
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// Past this many lines a "function" is more likely a whole file of
/// top-level code; the app falls back to the blame block.
const MAX_FUNCTION_LINES: u32 = 400;

/// git's built-in diff drivers, by file extension.
const BUILTIN_DRIVERS: &[(&str, &[&str])] = &[
    ("rust", &["rs"]),
    ("python", &["py", "pyi", "pyw"]),
    ("golang", &["go"]),
    (
        "cpp",
        &[
            "c", "h", "cc", "cpp", "cxx", "hh", "hpp", "hxx", "c++", "h++",
        ],
    ),
    ("java", &["java"]),
    ("csharp", &["cs"]),
    ("kotlin", &["kt", "kts"]),
    ("php", &["php"]),
    ("ruby", &["rb"]),
    ("perl", &["pl", "pm"]),
    ("bash", &["sh", "bash"]),
    ("objc", &["m", "mm"]),
    ("elixir", &["ex", "exs"]),
    ("css", &["css"]),
    ("html", &["html", "htm"]),
    ("markdown", &["md", "markdown"]),
    ("tex", &["tex"]),
    ("fortran", &["f", "f90", "f95", "for"]),
    ("pascal", &["pas"]),
    ("scheme", &["scm", "ss", "rkt"]),
    ("dts", &["dts", "dtsi"]),
];

/// The function around `line` (1-indexed) in `path` as of `at`, by git's
/// funcname rules for the file's diff driver. `None` when no function
/// header governs the line, the function is over 400 lines, or the line is
/// past the file's end.
///
/// # Errors
/// [`RunError`] from the runner: git missing or stopped, or `path` not in
/// `at`.
pub fn enclosing_function(
    repo: &RepoHandle,
    at: Oid,
    path: &Path,
    line: u32,
    stop: &dyn Stop,
) -> Result<Option<LineRange>, RunError> {
    let mut spec = format!("{at}:").into_bytes();
    spec.extend(path_bytes(path));
    let content = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Flag("blob"),
            GitArg::Ref(spec),
        ])
        .run(stop)?
        .stdout;
    let headers = headers_by_line(repo, path, &content, stop)?;
    Ok(function_at(&content, &headers, line))
}

/// Every line's governing funcname, index 0 = line 1; `None` where git
/// found none.
fn headers_by_line(
    repo: &RepoHandle,
    path: &Path,
    content: &[u8],
    stop: &dyn Stop,
) -> Result<Vec<Option<Vec<u8>>>, RunError> {
    let lines = split_lines(content);
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let scratch = tempfile::tempdir().map_err(RunError::Spawn)?;
    let (old_file, new_file) = write_pair(scratch.path(), path, &lines)?;

    let mut invocation = GitInvocation::new(repo);
    if let Some(attributes) = graft_attributes(repo, path, stop)? {
        let file = scratch.path().join("attributes");
        std::fs::write(&file, attributes).map_err(RunError::Spawn)?;
        invocation = invocation
            .arg(GitArg::Flag("-c"))
            .arg(GitArg::Glued("core.attributesFile=", path_bytes(&file)));
    }
    let raw = invocation
        .args([
            GitArg::Flag("diff"),
            GitArg::Flag("--no-index"),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-ext-diff"),
            GitArg::Flag("--src-prefix=a/"),
            GitArg::Flag("--dst-prefix=b/"),
            GitArg::Flag("-U0"),
            GitArg::Separator,
            GitArg::Path(path_bytes(&old_file)),
            GitArg::Path(path_bytes(&new_file)),
        ])
        .run_raw(stop)?;
    // `--no-index` exits 1 when the files differ, which they always do.
    match raw.code {
        Some(0 | 1) => {}
        Some(status) => {
            return Err(RunError::Exited {
                status,
                stderr_first_line: crate::commit_graph::first_line(&raw.output.stderr),
                command: "git diff --no-index -U0 (function headers)".to_owned(),
            });
        }
        None => return Err(RunError::Stopped),
    }
    let diffs = parse_unified(&raw.output.stdout).map_err(|error| {
        RunError::Unparsable(format!(
            "diff --no-index at line {}: {}",
            error.line, error.what
        ))
    })?;
    let mut headers = vec![None; lines.len()];
    // A binary file gives no hunks: every line stays `None`.
    for hunk in diffs.iter().flat_map(|diff| &diff.hunks) {
        // An insert after old line N is `@@ -N,0 +… @@ <header>`.
        let Some(slot) = (hunk.old_start as usize)
            .checked_sub(1)
            .and_then(|index| headers.get_mut(index))
        else {
            continue;
        };
        if !hunk.heading.is_empty() {
            *slot = Some(hunk.heading.clone());
        }
    }
    Ok(headers)
}

/// The file twice under `scratch`, keeping `path`'s base name: `a/<name>`
/// as it is, `b/<name>` with a unique marker line after every line.
fn write_pair(
    scratch: &Path,
    path: &Path,
    lines: &[&[u8]],
) -> Result<(std::path::PathBuf, std::path::PathBuf), RunError> {
    let name = path
        .file_name()
        .map_or_else(|| "file".into(), ToOwned::to_owned);
    let (old, new) = (scratch.join("a"), scratch.join("b"));
    let (old_file, new_file) = (old.join(&name), new.join(&name));
    let mut plain = Vec::new();
    let mut marked = Vec::new();
    for (index, text) in lines.iter().enumerate() {
        plain.extend_from_slice(text);
        plain.push(b'\n');
        marked.extend_from_slice(text);
        marked.push(b'\n');
        marked.extend_from_slice(format!("\u{1}graft-marker-{index}\n").as_bytes());
    }
    for (dir, file, bytes) in [(&old, &old_file, &plain), (&new, &new_file, &marked)] {
        std::fs::create_dir_all(dir).map_err(RunError::Spawn)?;
        std::fs::write(file, bytes).map_err(RunError::Spawn)?;
    }
    Ok((old_file, new_file))
}

/// What to pass as `core.attributesFile`, if anything: never when the user
/// has one; git's built-in driver for the extension when the repository
/// names none; the repository's driver when it names one.
fn graft_attributes(
    repo: &RepoHandle,
    path: &Path,
    stop: &dyn Stop,
) -> Result<Option<String>, RunError> {
    if user_attributes_file(repo, stop)? {
        return Ok(None);
    }
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("check-attr"),
            GitArg::Flag("diff"),
            GitArg::Separator,
        ])
        .arg(GitArg::Path(path_bytes(path)))
        .run(stop)?;
    // `<path>: diff: <value>`; the value is the text after the last ": ".
    let text = String::from_utf8_lossy(&output.stdout);
    let value = text.trim_end().rsplit(": ").next().unwrap_or("unspecified");
    let driver = match value {
        // `-diff` (binary) or plain `diff` (text, no driver): git's own
        // answer stands.
        "unset" | "set" => return Ok(None),
        "unspecified" => {
            let extension = path
                .extension()
                .map(|ext| ext.to_string_lossy().to_ascii_lowercase());
            let Some(driver) = extension.and_then(|ext| builtin_driver(&ext)) else {
                return Ok(None);
            };
            driver
        }
        named => named,
    };
    Ok(Some(format!("* diff={driver}\n")))
}

/// Whether the user (any config level git reads) sets `core.attributesFile`.
fn user_attributes_file(repo: &RepoHandle, stop: &dyn Stop) -> Result<bool, RunError> {
    match GitInvocation::new(repo)
        .args([
            GitArg::Flag("config"),
            GitArg::Flag("--get"),
            GitArg::Flag("core.attributesFile"),
        ])
        .run(stop)
    {
        Ok(_) => Ok(true),
        // `git config --get` exits 1 for a key that is not set.
        Err(RunError::Exited { status: 1, .. }) => Ok(false),
        Err(other) => Err(other),
    }
}

fn builtin_driver(extension: &str) -> Option<&'static str> {
    BUILTIN_DRIVERS
        .iter()
        .find(|(_, extensions)| extensions.contains(&extension))
        .map(|(driver, _)| *driver)
}

/// The file's lines without their `\n`; a final line without one counts.
fn split_lines(content: &[u8]) -> Vec<&[u8]> {
    let mut lines: Vec<&[u8]> = content.split(|&b| b == b'\n').collect();
    if lines.last().is_some_and(|last| last.is_empty()) {
        lines.pop();
    }
    lines
}

/// The range [`enclosing_function`] answers, from the per-line headers.
/// Pure.
fn function_at(content: &[u8], headers: &[Option<Vec<u8>>], line: u32) -> Option<LineRange> {
    let lines = split_lines(content);
    let at = (line as usize).checked_sub(1)?;
    let header = headers.get(at)?.as_ref()?;
    // The function starts where its header first governs.
    let mut start = at;
    while start > 0 && headers[start - 1].as_ref() == Some(header) {
        start -= 1;
    }
    let indent = indentation(lines.get(start)?);
    let is_header_line =
        |index: usize| headers[index].is_some() && headers[index] != headers[index - 1];
    let mut end = (start + 1..headers.len())
        .find(|&index| is_header_line(index) && indentation(lines[index]) <= indent)
        .map_or(lines.len() - 1, |next| next - 1);
    while end > at && lines[end].iter().all(u8::is_ascii_whitespace) {
        end -= 1;
    }
    let (start, end) = (u32::try_from(start + 1).ok()?, u32::try_from(end + 1).ok()?);
    if end - start + 1 > MAX_FUNCTION_LINES {
        return None;
    }
    Some(LineRange::Function {
        name: String::from_utf8_lossy(header).trim().to_owned(),
        start,
        end,
    })
}

fn indentation(line: &[u8]) -> usize {
    line.iter()
        .take_while(|b| **b == b' ' || **b == b'\t')
        .count()
}

/// A working-tree line's number in `HEAD`, walking the worktree-vs-`HEAD`
/// hunks (old side `HEAD`, new side the working tree). `None` for a line
/// inside an added run: it has no `HEAD` counterpart. Pure.
#[must_use]
pub fn map_to_head(worktree_vs_head: &[Hunk], line: u32) -> Option<u32> {
    let mut delta: i64 = 0; // HEAD line minus working-tree line, so far
    for hunk in worktree_vs_head {
        let covers = hunk.new_lines > 0
            && line >= hunk.new_start
            && line < hunk.new_start.saturating_add(hunk.new_lines);
        if covers {
            let found = hunk.lines.iter().find(|l| l.new_no == Some(line))?;
            return match found.kind {
                LineKind::Context => found.old_no,
                LineKind::Added | LineKind::Removed => None,
            };
        }
        // A pure removal (`+N,0`) sits after new line N; lines up to N are
        // before it.
        let before = if hunk.new_lines == 0 {
            line <= hunk.new_start
        } else {
            line < hunk.new_start
        };
        if before {
            break;
        }
        delta += i64::from(hunk.old_lines) - i64::from(hunk.new_lines);
    }
    u32::try_from(i64::from(line) + delta)
        .ok()
        .filter(|&n| n > 0)
}
