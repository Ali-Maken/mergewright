//! The remote under test: a bare and its work clone
//! ([`LocalRemote`]), optionally served over Basic-auth HTTP or replaced by
//! a socket that never answers, with the machine's own git config out of the
//! app's reach.

use std::path::Path;

use mergewright_fixtures::Shape;
use mergewright_fixtures::remote::{HttpRemote, HungRemote, LocalRemote};

use crate::repo_truth::{neutralise_global_config, run_git};

/// The HTTP remote's user; asked in the open when the URL does not name it.
pub const USER: &str = "tester";
/// The HTTP remote's password. Typed into the masked prompt, never echoed:
/// [`no_secret_leaked`] proves it, and `pty-walk-m4.py` greps every capture
/// for it.
pub const PASSWORD: &str = "correct-horse-battery-staple";
/// The password git is given first in `wrong-password-then-right`.
pub const WRONG: &str = "not-the-password";
/// Every string that must appear in no capture, status line, event or
/// journal — the pty script keeps the same list.
pub const SECRETS: [&str; 2] = [PASSWORD, WRONG];

/// Which remote the work clone's `origin` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteKind {
    /// The bare over `file://` — the transport `LocalRemote` gives.
    Local,
    /// The same bare over smart HTTP with Basic auth (`USER`/`PASSWORD`);
    /// `anonymous` leaves the user out of the URL so git asks for it first.
    Http { anonymous: bool },
    /// A loopback port that accepts and never answers.
    Hung,
}

/// The remote a scenario runs against, alive for as long as it runs.
pub struct Fixture {
    /// The bare, the work clone and a teammate's clone.
    pub local: LocalRemote,
    /// Serving the bare, when the scenario is over HTTP.
    pub http: Option<HttpRemote>,
    /// Never answering, when the scenario is about a hang.
    pub hung: Option<HungRemote>,
}

impl Fixture {
    /// Build `shape` as a remote of `kind`, and put the machine's own git
    /// config out of the app's reach in the work clone.
    pub fn build(shape: Shape, kind: RemoteKind) -> Result<Self, String> {
        let local = LocalRemote::new(shape).map_err(|why| format!("fixture: {why:?}"))?;
        // The app under test runs in this process with the user's real git
        // config; the work clone overrides what would change a write's
        // meaning (m3_walk), and blanks any credential helper: only the keys
        // this walk presses may answer git (credential_by_key).
        neutralise_global_config(local.work())?;
        run_git(
            local.work(),
            &["config", "--local", "credential.helper", ""],
        )?;
        let mut fixture = Fixture {
            local,
            http: None,
            hung: None,
        };
        match kind {
            RemoteKind::Local => {}
            RemoteKind::Http { anonymous } => {
                let http = HttpRemote::serve(fixture.local.bare(), USER, PASSWORD)
                    .map_err(|why| format!("the HTTP remote: {why}"))?;
                let url = if anonymous {
                    http.url_anonymous()
                } else {
                    http.url()
                };
                fixture.git(fixture.work(), &["remote", "set-url", "origin", &url])?;
                fixture.http = Some(http);
            }
            RemoteKind::Hung => {
                let hung = HungRemote::listen().map_err(|why| format!("the hung remote: {why}"))?;
                fixture.git(
                    fixture.work(),
                    &["remote", "set-url", "origin", &hung.url()],
                )?;
                fixture.hung = Some(hung);
            }
        }
        Ok(fixture)
    }

    /// The clone the app opens.
    pub fn work(&self) -> &Path {
        self.local.work()
    }

    /// The remote itself.
    pub fn bare(&self) -> &Path {
        self.local.bare()
    }

    /// The scratch root: the bare, the work clone and the teammate's clone
    /// are its children, and so is any clone a scenario adds.
    pub fn root(&self) -> &Path {
        self.work()
            .parent()
            .expect("the work clone lives inside the fixture's scratch")
    }

    /// The default branch's short name (`main` for the corpus shapes).
    pub fn branch(&self) -> &str {
        self.local.default_branch()
    }

    /// `refs/heads/<branch>`.
    pub fn main_ref(&self) -> String {
        format!("refs/heads/{}", self.branch())
    }

    /// `refs/remotes/origin/<branch>`.
    pub fn tracking_ref(&self) -> String {
        format!("refs/remotes/origin/{}", self.branch())
    }

    /// Hermetic `git` in `dir` (the fixture's own runner), trimmed stdout.
    pub fn git(&self, dir: &Path, args: &[&str]) -> Result<String, String> {
        self.local
            .git(dir, args)
            .map_err(|why| format!("git {args:?} in {}: {why}", dir.display()))
    }

    /// A teammate's commit that **changes** `file` (a tracked file of the
    /// shape) to `content` and pushes it, so the work clone falls behind on
    /// a path it has. `LocalRemote::commit_on_remote` adds a new file, which
    /// is not what a dirty-file refusal is about.
    pub fn change_on_remote(&self, file: &str, content: &str) -> Result<String, String> {
        let mate = self.root().join("mate-change");
        if !mate.exists() {
            let bare = self.bare().to_string_lossy().into_owned();
            let mate_arg = mate.to_string_lossy().into_owned();
            self.git(self.root(), &["clone", "-q", &bare, &mate_arg])?;
        }
        self.git(&mate, &["pull", "-q", "--ff-only", "origin", self.branch()])?;
        std::fs::write(mate.join(file), content).map_err(|why| format!("{file}: {why}"))?;
        self.git(&mate, &["add", "--", file])?;
        self.git(&mate, &["commit", "-q", "-m", &format!("change {file}")])?;
        self.git(
            &mate,
            &["push", "-q", "origin", &format!("HEAD:{}", self.main_ref())],
        )?;
        self.git(&mate, &["rev-parse", "HEAD"])
    }

    /// A teammate's commit of `count` distinct files of `size` random-ish
    /// bytes — enough objects that git reports progress while the work
    /// clone receives them.
    pub fn many_objects_on_remote(&self, count: usize, size: usize) -> Result<String, String> {
        let mate = self.root().join("mate-many");
        let bare = self.bare().to_string_lossy().into_owned();
        let mate_arg = mate.to_string_lossy().into_owned();
        self.git(self.root(), &["clone", "-q", &bare, &mate_arg])?;
        let dir = mate.join("bulk");
        std::fs::create_dir_all(&dir).map_err(|why| format!("bulk: {why}"))?;
        // A cheap generator with no repetition git could delta away.
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for n in 0..count {
            let mut bytes = Vec::with_capacity(size);
            while bytes.len() < size {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                bytes.extend_from_slice(&seed.to_le_bytes());
            }
            std::fs::write(dir.join(format!("{n}.bin")), &bytes)
                .map_err(|why| format!("bulk/{n}.bin: {why}"))?;
        }
        self.git(&mate, &["add", "--", "bulk"])?;
        self.git(&mate, &["commit", "-q", "-m", "bulk"])?;
        self.git(
            &mate,
            &["push", "-q", "origin", &format!("HEAD:{}", self.main_ref())],
        )?;
        self.git(&mate, &["rev-parse", "HEAD"])
    }

    /// The first tracked file of the shape, the one a teammate's change
    /// will touch.
    pub fn a_tracked_file(&self) -> Result<String, String> {
        run_git(self.work(), &["ls-files"])?
            .lines()
            .next()
            .map(str::to_owned)
            .ok_or_else(|| "the shape has no tracked file".to_owned())
    }
}
