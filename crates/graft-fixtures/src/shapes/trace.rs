//! The session repository (m7-power-tools/18): one Rust file written by four
//! people over fourteen months, renamed once, reformatted once — what every
//! blame and history walk in M7 runs against, matching the design note's
//! mock-ups.
//!
//! `src/auth/session.rs` at `main`, 60 lines. Who last touched what, as
//! `git blame -w -M` has it (with `--ignore-revs-file
//! .git-blame-ignore-revs` too, the Rustfmt row's lines pass to Ana):
//!
//! | lines | who | commit |
//! |---|---|---|
//! | 1, 4–9, 11–13, 16, 30, 34, 38–39, 41–56, 58–60 | Ana | "Add session" (420 days before `CORPUS_NOW`) |
//! | 17–18, 20–21, 24–25, 27–29 | Tom | "Add token refresh" (300 days) — `refresh_token`, a method inside `impl Session` |
//! | 19, 22–23 | Lina | "Retry refresh once on 401" (210 days): adds `let mut attempt = 0;` |
//! | 31–33, 35–37 | Rustfmt | "rustfmt" (150 days): two one-line methods split in three; listed in `.git-blame-ignore-revs` |
//! | 2, 10, 14–15, 40, 57 | Priya | "Move session TTL to config" (120 days): also renames `src/session.rs` → `src/auth/session.rs` |
//! | 3, 26 | you (the fixture identity) | "Handle expired tokens" (an hour before `CORPUS_NOW`) |
//!
//! (Priya's line 2 is `use crate::config::Config;`; Ana's `use crate::http`
//! moved to line 4.) `refresh_token` sits at lines 17–27 in Tom's commit,
//! 17–28 after Lina's, and 18–29 at `main`: "you" added an import above it,
//! so the range moves with the code. Line 27 is over 150 characters.
//!
//! Also: `assets/pic.bin` (binary, committed by Ana), `tools/rotate.py` (a
//! Python file with a `def`), and an untracked `src/new.rs` in the working
//! tree. Not in [`Shape::ALL`](crate::Shape::ALL): the untracked file makes
//! the working tree unclean, which the `ALL` loops assume it is not.

use super::{BuildError, MAIN, Site, file};
use crate::stream::Stream;

const DAY: u64 = 24 * 60 * 60;

const ANA: &str = "Ana <ana@example.invalid>";
const TOM: &str = "Tom <tom@example.invalid>";
const LINA: &str = "Lina <lina@example.invalid>";
const RUSTFMT: &str = "Rustfmt <rustfmt@example.invalid>";
const PRIYA: &str = "Priya <priya@example.invalid>";

/// `when` days before the corpus's fixed clock.
#[allow(
    clippy::cast_sign_loss,
    reason = "CORPUS_NOW is a positive constant well past every offset used here"
)]
fn days_ago(days: u64) -> u64 {
    crate::CORPUS_NOW as u64 - days * DAY
}

/// The long line (27 at `main`), over 150 characters, so the blame trailer
/// has to fall back.
const LONG: &str = "            return self.record_refresh(&resp, \"refreshed after the server answered 401 on the first request; the retry budget is spent\");";

/// Lines joined, each ending in `\n`.
fn text(lines: &[&str]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Ana's file, before anything else: the struct, `new`, two one-line
/// accessors rustfmt later splits, and a tail.
fn ana() -> Vec<&'static str> {
    vec![
        "use std::time::{Duration, Instant};",
        "use crate::http::{Client, Result};",
        "",
        "const SESSION_TTL: Duration = Duration::from_secs(3600);",
        "",
        "pub struct Session {",
        "    token: String,",
        "    refresh: String,",
        "    issued: Instant,",
        "}",
        "",
        "impl Session {",
        "    pub fn new(token: String, refresh: String) -> Self {",
        "        Session { token, refresh, issued: Instant::now() }",
        "    }",
        "",
        "    pub fn token(&self)->&str { &self.token }",
        "",
        "    pub fn age(&self)->Duration { self.issued.elapsed() }",
        "",
        "    pub fn is_expired(&self) -> bool {",
        "        self.age() > SESSION_TTL",
        "    }",
        "}",
        "",
        "impl Drop for Session {",
        "    fn drop(&mut self) {",
        "        self.token.clear();",
        "        self.refresh.clear();",
        "    }",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn a_new_session_is_not_expired() {",
        "        let session = Session::new(\"t\".into(), \"r\".into());",
        "        assert!(!session.is_expired());",
        "    }",
        "}",
    ]
}

/// Tom's `refresh_token`, with its doc comment, inserted after `new`.
fn tom(mut lines: Vec<&'static str>) -> Vec<&'static str> {
    let at = 15; // after `new`'s closing brace (line 15)
    let refresh = [
        "    /// Refresh the token when the server says it expired.",
        "    pub fn refresh_token(&mut self, client: &Client) -> Result<()> {",
        "        loop {",
        "            let resp = client.refresh(&self.token)?;",
        "            if resp.status() == 401 {",
        "                self.back_off();",
        "                continue;",
        "            }",
        "            self.token = resp.token()?;",
        LONG,
        "        }",
        "    }",
    ];
    lines.splice(at..at, refresh);
    lines
}

/// Lina's retry: a counter before the loop, the 401 check and its body
/// changed.
fn lina(mut lines: Vec<&'static str>) -> Vec<&'static str> {
    // `refresh_token` opens at line 17 (index 16).
    lines.insert(17, "        let mut attempt = 0;");
    lines[20] = "            if resp.status() == 401 && attempt == 0 {";
    lines[21] = "                attempt += 1;";
    lines
}

/// rustfmt: the two one-line accessors split into three lines each.
fn rustfmt(lines: Vec<&'static str>) -> Vec<&'static str> {
    let mut out = Vec::with_capacity(lines.len() + 4);
    for line in lines {
        match line {
            "    pub fn token(&self)->&str { &self.token }" => out.extend([
                "    pub fn token(&self) -> &str {",
                "        &self.token",
                "    }",
            ]),
            "    pub fn age(&self)->Duration { self.issued.elapsed() }" => out.extend([
                "    pub fn age(&self) -> Duration {",
                "        self.issued.elapsed()",
                "    }",
            ]),
            other => out.push(other),
        }
    }
    out
}

/// Priya: the TTL moves from a constant to config.
fn priya(lines: Vec<&'static str>) -> Vec<&'static str> {
    let mut out = Vec::with_capacity(lines.len());
    let mut skip_blank = false;
    for line in lines {
        if skip_blank {
            skip_blank = false;
            if line.is_empty() {
                continue;
            }
        }
        match line {
            "use crate::http::{Client, Result};" => {
                out.push("use crate::config::Config;");
                out.push(line);
            }
            "const SESSION_TTL: Duration = Duration::from_secs(3600);" => skip_blank = true,
            "    issued: Instant," => {
                out.push(line);
                out.push("    ttl: Duration,");
            }
            "    pub fn new(token: String, refresh: String) -> Self {" => {
                out.push("    pub fn new(token: String, refresh: String, config: &Config) -> Self {");
            }
            "        Session { token, refresh, issued: Instant::now() }" => out.push(
                "        Session { token, refresh, issued: Instant::now(), ttl: config.session_ttl }",
            ),
            "        self.age() > SESSION_TTL" => out.push("        self.age() > self.ttl"),
            "        let session = Session::new(\"t\".into(), \"r\".into());" => out.push(
                "        let session = Session::new(\"t\".into(), \"r\".into(), &Config::default());",
            ),
            other => out.push(other),
        }
    }
    out
}

/// "you": expired tokens get their own error.
fn you(lines: Vec<&'static str>) -> Vec<&'static str> {
    let mut out = Vec::with_capacity(lines.len() + 1);
    for line in lines {
        match line {
            "use crate::http::{Client, Result};" => {
                out.push("use crate::error::Expired;");
                out.push(line);
            }
            "            self.token = resp.token()?;" => {
                out.push("            self.token = resp.token().map_err(Expired::from)?;");
            }
            other => out.push(other),
        }
    }
    out
}

/// Ana's, Tom's, Lina's, rustfmt's and Priya's commits as one stream, and
/// Priya's version of the file (the one "you" then change).
fn four_people() -> (Stream, Vec<&'static str>) {
    let v_ana = ana();
    let v_tom = tom(v_ana.clone());
    let v_lina = lina(v_tom.clone());
    let v_rustfmt = rustfmt(v_lina.clone());
    let v_priya = priya(v_rustfmt.clone());
    let old: &[u8] = b"src/session.rs";
    let new: &[u8] = b"src/auth/session.rs";

    let mut stream = Stream::new();
    let root = stream.commit_full(
        (ANA, days_ago(420)),
        MAIN,
        b"Add session\n",
        &[],
        &[
            file(old, text(&v_ana).as_bytes()),
            file(
                b"assets/pic.bin",
                b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01",
            ),
            file(b"tools/rotate.py", PYTHON.as_bytes()),
        ],
        &[],
    );
    let steps = [
        (TOM, 300, &b"Add token refresh\n"[..], old, &v_tom),
        (LINA, 210, b"Retry refresh once on 401\n", old, &v_lina),
        (RUSTFMT, 150, b"rustfmt\n", old, &v_rustfmt),
        (PRIYA, 120, b"Move session TTL to config\n", new, &v_priya),
    ];
    let mut parent = root;
    for (who, days, message, path, lines) in steps {
        // Priya's commit renames: the old path goes as the new one comes.
        let deleted: &[&[u8]] = if path == new { &[old] } else { &[] };
        parent = stream.commit_full(
            (who, days_ago(days)),
            MAIN,
            message,
            &[parent],
            &[file(path, text(lines).as_bytes())],
            deleted,
        );
    }
    (stream, v_priya)
}

const PYTHON: &str = "import time\n\n\ndef rotate(keys):\n    fresh = [k for k in keys if not k.expired]\n    fresh.sort(key=lambda k: k.issued)\n    return fresh\n\n\ndef main():\n    print(rotate([]))\n";

/// The trace shape. See the module doc.
pub(super) fn session(site: &Site<'_>) -> Result<(), BuildError> {
    let (stream, v_priya) = four_people();
    site.import(stream)?;
    site.checkout_main()?;

    // The ignore-revs file names the rustfmt commit, whose id exists only
    // now; it lands in "you"'s commit, made by plain git at a fixed date so
    // the id is the same every build.
    let fmt_id = site.git(&["rev-parse", "main~1"])?;
    let fmt_id = String::from_utf8_lossy(&fmt_id).trim().to_owned();
    site.write(
        ".git-blame-ignore-revs",
        &format!("# rustfmt, the whole tree\n{fmt_id}\n"),
    )?;
    site.write("src/auth/session.rs", &text(&you(v_priya)))?;
    let date = format!("@{} +0000", crate::CORPUS_NOW - 3600);
    site.git_at(&["add", "-A"], &date)?;
    site.git_at(&["commit", "-q", "-m", "Handle expired tokens"], &date)?;

    // Untracked, after the last commit.
    site.write("src/new.rs", "pub fn fresh() {}\n")
}
