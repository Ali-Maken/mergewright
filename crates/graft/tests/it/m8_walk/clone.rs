//! J15 `clone` (m8-polish-and-ship/18c, rulings R6): `c` on the empty
//! state, the URL, `⏎` — on the real binary over a pty, so the clone job
//! runs on the empty state's own pool and `drive` carries the landed clone
//! to its graph (18b's glue, unverified until here). Remotes are local
//! only: a `file://` bare, loopback HTTP with Basic auth, a port that never
//! answers. Git's system and global config are off for every run
//! ([`Pty::spawn_without_git_config`]).
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::path::{Path, PathBuf};

use graft_fixtures::Shape;
use graft_fixtures::remote::{HttpRemote, HungRemote, LocalRemote};

use crate::pty::{Isolated, Pty, Screen};

const USER: &str = "tester";
const PASSWORD: &str = "clone-walk-horse-battery";

/// J15's inputs as the fence counts them: `c`, the URL typed (one input,
/// `Type`), `⏎`.
fn assert_within_the_note(inputs: usize) {
    crate::m8_walk::within_the_note_taking("clone", inputs, "(c, the URL, ⏎)");
}

/// `~/code` under the isolated home, empty: where Graft starts and where
/// the clone lands, drawn `~/code/…` so captures hold no machine path.
fn code(isolated: &Isolated) -> PathBuf {
    let code = isolated.home().join("code");
    std::fs::create_dir_all(&code).expect("~/code");
    code
}

fn start(isolated: &Isolated, folder: &Path) -> Pty {
    let pty = Pty::spawn_without_git_config(isolated, folder, isolated.root.path(), (120, 36));
    pty.wait_for("the empty state", |s| {
        s.has("no repository here") && s.has("c clone\u{2026}")
    });
    pty
}

/// `c`, then the URL typed: the prompt with its `→` line.
fn type_url(pty: &mut Pty, url: &str, into: &str) -> Screen {
    pty.send(b"c");
    pty.wait_for("the clone prompt", |s| {
        s.has("clone \u{203a}") && s.has("\u{23ce} clone \u{b7} esc cancel")
    });
    pty.send(url.as_bytes());
    // A long URL scrolls in the field: its end, then the cursor, are what
    // is in view.
    let tail: String = {
        let chars: Vec<char> = url.chars().collect();
        chars[chars.len().saturating_sub(40)..].iter().collect()
    };
    pty.wait_for("the URL and where it goes", |s| {
        s.has("clone \u{203a} ")
            && s.has(&format!("{tail}\u{258f}"))
            && s.has(&format!("\u{2192} {into}"))
    })
}

fn quit(mut pty: Pty) -> Vec<u8> {
    pty.send(b"q");
    let (status, stderr, output) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
    assert!(stderr.is_empty(), "nothing on stderr: {stderr:?}");
    output
}

/// `esc` then `q`: out of a prompt, then out of Graft.
fn close_and_quit(mut pty: Pty) -> Vec<u8> {
    pty.send(b"\x1b");
    pty.wait_for("the list again", |s| s.has("c clone\u{2026}"));
    quit(pty)
}

/// The landed clone's graph, cursor on HEAD's row.
fn on_the_graph_at_head(pty: &Pty, name: &str, subject: &str) -> Screen {
    let screen = pty.wait_for("the clone's graph", |s| {
        s.lines()[0].contains(name) && s.has(subject) && !s.has("no repository here")
    });
    let cursor = screen
        .lines()
        .into_iter()
        .find(|line| line.contains('\u{25b8}'))
        .unwrap_or_else(|| panic!("no cursor row:\n{}", screen.text()));
    assert!(
        cursor.contains(subject),
        "cursor not on HEAD ({subject}): {cursor:?}\n{}",
        screen.text()
    );
    screen
}

fn head_subject(remote: &LocalRemote) -> String {
    remote
        .git(remote.bare(), &["log", "-1", "--format=%s"])
        .expect("the bare's HEAD")
        .trim()
        .to_owned()
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("listed")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// J15 `clone` = 3: `c`, the URL, `⏎` — a `file://` bare lands as that
/// repository's graph, cursor on HEAD, and is remembered.
#[test]
fn clone_from_the_empty_state() {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let subject = head_subject(&remote);
    let isolated = Isolated::new();
    let code = code(&isolated);
    let mut pty = start(&isolated, &code);

    let url = remote.url();
    type_url(&mut pty, &url, "~/code/remote");
    pty.send(b"\r");
    on_the_graph_at_head(&pty, "remote", &subject);
    assert_within_the_note(3);

    quit(pty);
    let landed = code.join("remote");
    assert!(landed.join(".git").is_dir(), "the clone is a repository");
    let recent = std::fs::read_to_string(isolated.recent_file()).expect("recents written");
    let newest = recent.lines().next().expect("one line");
    assert_eq!(
        Path::new(newest.split_once('\t').expect("secs\\tpath").1),
        landed,
        "the clone is remembered"
    );
}

/// The capture names: the port is the one thing a loopback server
/// changes run to run, and the masked field's first glyph is the
/// keystroke pulse, random per prompt by design (ADR-0038 §2).
fn steady(screen: &Screen, port: Option<u16>) -> String {
    let mut text = screen.text();
    if let Some(port) = port {
        let port = port.to_string();
        text = text.replace(&port, &"0".repeat(port.len()));
    }
    text.replace('\u{25e6}', "\u{2022}")
}

fn shot(shots: &mut graft_fixtures::Captures, name: &str, right: &str, text: &str) {
    shots.check(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots/m8")
            .join(name),
        format!(
            "m8-polish-and-ship/18c: what right looks like — {right} \
             Colour unverified (rule 5).\n\n{text}\n"
        ),
    );
}

/// HTTP Basic: git's question opens the credential box over the empty
/// state, the password typed there lands the clone, and the password is in
/// no frame, not on stderr, and nowhere in the new repository.
#[test]
fn clone_asks_for_a_password() {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let subject = head_subject(&remote);
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).expect("loopback HTTP");
    let isolated = Isolated::new();
    let code = code(&isolated);
    let mut pty = start(&isolated, &code);

    type_url(&mut pty, &http.url(), "~/code/repo");
    pty.send(b"\r");
    let asked = pty.wait_for("the credential box", |s| {
        s.has("Credentials \u{2014} 127.0.0.1") && s.has("\u{23ce} send \u{b7} esc cancel")
    });
    assert!(
        asked.has("Password for") && asked.has("esc cancel the clone"),
        "{}",
        asked.text()
    );
    let mut shots = graft_fixtures::Captures::new();
    shot(
        &mut shots,
        "clone-credential-120x36.txt",
        "git asked for a password while cloning: the M4 credential box over the empty state, \
         its band cleared full width, title `Credentials — 127.0.0.1:<port>`, git's question, \
         the masked field, `typed here only · never stored by Tributary`, footer `⏎ send ⌃U clear \
         esc cancel the clone`; the bar row reads the box's keys `⏎ send · esc cancel`. Port \
         zeroed, pulse glyph drawn •.",
        &steady(&asked, Some(http.port())),
    );
    shots.finish();

    pty.send(PASSWORD.as_bytes());
    pty.send(b"\r");
    on_the_graph_at_head(&pty, "repo", &subject);
    let output = quit(pty);

    let secret = PASSWORD.as_bytes();
    let leaks = |bytes: &[u8]| bytes.windows(secret.len()).any(|w| w == secret);
    assert!(!leaks(&output), "the password reached a frame");
    let landed = code.join("repo");
    assert!(landed.join(".git").is_dir(), "the clone landed");
    for file in walk(&landed.join(".git")) {
        let bytes = std::fs::read(&file).expect("readable");
        assert!(!leaks(&bytes), "the password is in {}", file.display());
    }
    let (authorised, _) = http.requests();
    assert!(authorised >= 1, "the server saw the password");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).expect("listed") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// R6 `⏎`, no name: the prompt stays with the reason; nothing ran.
#[test]
fn bad_url_stays() {
    let isolated = Isolated::new();
    let code = code(&isolated);
    let mut pty = start(&isolated, &code);
    type_url(&mut pty, "https://example.com/", "~/code/\u{2026}");
    pty.send(b"\r");
    let screen = pty.wait_for("the reason", |s| {
        s.has("can't name a folder from this URL") && s.has("\u{23ce} retry \u{b7} esc cancel")
    });
    assert!(
        screen.has("clone \u{203a} https://example.com/"),
        "URL kept"
    );
    close_and_quit(pty);
    assert_eq!(names_in(&code), Vec::<String>::new(), "nothing written");
}

/// R6 `⏎`, target non-empty: `<name> already exists here`; nothing ran, his
/// folder untouched.
#[test]
fn target_exists_refused() {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let isolated = Isolated::new();
    let code = code(&isolated);
    std::fs::create_dir(code.join("remote")).expect("his folder");
    std::fs::write(code.join("remote/notes.txt"), "his").expect("his file");
    let mut pty = start(&isolated, &code);
    type_url(&mut pty, &remote.url(), "~/code/remote");
    pty.send(b"\r");
    pty.wait_for("the reason", |s| {
        s.has("remote already exists here") && s.has("\u{23ce} retry \u{b7} esc cancel")
    });
    close_and_quit(pty);
    assert_eq!(names_in(&code.join("remote")), ["notes.txt"]);
}

/// R6 fails: the URL kept, `clone failed: <words>` on the prompt, and `⏎`
/// retries — here into the same failure, the second answer as the first.
#[test]
fn failure_keeps_the_url_and_retries() {
    let isolated = Isolated::new();
    let code = code(&isolated);
    let mut pty = start(&isolated, &code);
    let url = "file:///nonexistent/payments-service.git";
    let typed = type_url(&mut pty, url, "~/code/payments-service");
    let mut shots = graft_fixtures::Captures::new();
    shot(
        &mut shots,
        "clone-prompt-120x36.txt",
        "`c` then a URL typed: the boxed `clone ›` prompt under the list with the cursor `▏` \
         after the URL, and under it, muted, `→ ~/code/payments-service` — where it will land; \
         the header line now offers `c Clone…`; the bar row holds only the prompt's keys \
         `⏎ clone · esc cancel`.",
        &steady(&typed, None),
    );
    pty.send(b"\r");
    let failed = pty.wait_for("the failure", |s| {
        s.has("clone failed:") && s.has("\u{23ce} retry \u{b7} esc cancel")
    });
    assert!(failed.has(&format!("clone \u{203a} {url}")), "URL kept");
    shot(
        &mut shots,
        "clone-failed-120x36.txt",
        "the clone failed: the URL kept in the field, `→ ~/code/payments-service` under it, and \
         under that git's reason after `clone failed:` (Problem ink); the bar row offers \
         `⏎ retry · esc cancel`.",
        &steady(&failed, None),
    );
    shots.finish();

    close_and_quit(pty);
    assert_eq!(names_in(&code), Vec::<String>::new(), "nothing left");

    // Retry: the remote is not there, then it is; `⏎` on the kept URL runs
    // the clone again, and this time it lands.
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let subject = head_subject(&remote);
    let later = tempfile::tempdir().expect("scratch");
    let bare = later.path().join("later.git");
    let mut pty = start(&isolated, &code);
    let url = format!("file://{}", bare.display());
    type_url(&mut pty, &url, "~/code/later");
    pty.send(b"\r");
    pty.wait_for("the failure", |s| {
        s.has("clone failed:") && s.has("\u{23ce} retry \u{b7} esc cancel")
    });
    remote
        .git(
            later.path(),
            &[
                "clone",
                "--bare",
                "-q",
                &remote.bare().display().to_string(),
                "later.git",
            ],
        )
        .expect("the remote appears");
    pty.send(b"\r");
    on_the_graph_at_head(&pty, "later", &subject);
    quit(pty);
}

/// R6 `esc` while running: a clone against a port that never answers
/// stops, `clone stopped · nothing kept`, the URL kept, no folder.
#[test]
fn esc_mid_clone_stops_and_keeps_the_url() {
    let hung = HungRemote::listen().expect("a hung remote");
    let isolated = Isolated::new();
    let code = code(&isolated);
    let mut pty = start(&isolated, &code);
    let url = hung.url();
    type_url(&mut pty, &url, "~/code/repo");
    pty.send(b"\r");
    pty.wait_for("the clone running", |s| {
        s.has("\u{2192} ~/code/repo \u{b7} connecting\u{2026}") && s.has("esc cancels")
    });
    assert!(
        hung.wait_for_connection(std::time::Duration::from_secs(20)),
        "git never reached the hung remote"
    );
    // Running: keys but `esc` are swallowed.
    pty.send(b"x");
    pty.send(b"\x1b");
    let stopped = pty.wait_for("the stop", |s| {
        s.has("clone stopped \u{b7} nothing kept") && s.has("\u{23ce} retry \u{b7} esc cancel")
    });
    assert!(stopped.has(&format!("clone \u{203a} {url}")), "URL kept");
    assert!(
        !stopped.has(&format!("{url}x")),
        "x was typed while running"
    );
    close_and_quit(pty);
    assert!(!code.join("repo").exists(), "no folder left");
}

/// R6 `esc`, not running: the prompt closes, the list as before, nothing
/// written, nothing remembered.
#[test]
fn esc_closes_the_prompt() {
    let isolated = Isolated::new();
    let code = code(&isolated);
    let mut pty = start(&isolated, &code);
    type_url(&mut pty, "https://example.com/acme/x.git", "~/code/x");
    close_and_quit(pty);
    assert_eq!(names_in(&code), Vec::<String>::new());
    assert!(!isolated.recent_file().exists(), "nothing remembered");
}
