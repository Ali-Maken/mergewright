//! Generates the synthetic history's `fast-import` stream: trunk
//! runs, topic branches, tags, and realistic commit content.

use std::io;

use crate::stream::{Mark, Stream};

use super::{
    ANNOTATED_TAG_EVERY, CommitSize, DRAIN_EVERY, KEEP_REF_EVERY, LIGHT_TAG_EVERY, MAIN,
    OPEN_EVERY, SCRATCH_BRANCH, TOPIC_RUN, TRUNK_RUN,
};

/// The history under construction: a trunk, a topic branch at a time, and a
/// count of what has been emitted so far.
pub(super) struct History {
    stream: Stream,
    /// How many commits are still to be emitted.
    left: u32,
    /// How many have been emitted, which is also what tags are named after.
    done: u32,
    /// How many commits have gone into the buffer since the last drain.
    buffered: u32,
    tip: Option<Mark>,
    cycle: u32,
    size: CommitSize,
}

impl History {
    pub(super) fn new(commits: u32, size: CommitSize) -> Self {
        History {
            stream: Stream::new(),
            left: commits,
            done: 0,
            buffered: 0,
            tip: None,
            cycle: 0,
            size,
        }
    }

    pub(super) fn write(&mut self, out: &mut impl io::Write) -> Result<(), io::Error> {
        while self.left > 0 {
            self.trunk_run(out)?;
            self.topic_cycle(out)?;
        }
        // `take` rather than a field read: the stream is finished exactly
        // once, and the borrow checker should be the one saying so.
        std::mem::take(&mut self.stream).finish_into(out)
    }

    /// A run of trunk commits, or fewer if the budget runs out first.
    fn trunk_run(&mut self, out: &mut impl io::Write) -> Result<(), io::Error> {
        for _ in 0..TRUNK_RUN {
            if self.left == 0 {
                return Ok(());
            }
            let content = format!("trunk {}\n", self.done + 1);
            let message = format!("Trunk commit {}\n", self.done + 1);
            let parents: Vec<Mark> = self.tip.into_iter().collect();
            let (identity, message) = self.dressed(message);
            self.tip = Some(self.stream.commit_by(
                &identity,
                MAIN,
                message.as_bytes(),
                &parents,
                &[(b"trunk.txt", content.as_bytes())],
            ));
            self.counted();
            self.tag();
            self.maybe_drain(out)?;
        }
        Ok(())
    }

    /// One topic branch: three commits and, unless this is one of the open
    /// ones, a merge back into the trunk.
    ///
    /// Skipped entirely when the remaining budget cannot hold a whole one, so
    /// that a history never ends half-way through a branch. The caller's loop
    /// then spends what is left on trunk commits.
    fn topic_cycle(&mut self, out: &mut impl io::Write) -> Result<(), io::Error> {
        let open = self.cycle % OPEN_EVERY == OPEN_EVERY - 1;
        let needed = TOPIC_RUN + u32::from(!open);
        let Some(base) = self.tip else { return Ok(()) };
        if self.left < needed {
            return Ok(());
        }

        // A branch that keeps its ref is named after its cycle; the rest
        // share one scratch name that is deleted after the import, which is
        // what a repository whose merged branches were tidied away looks
        // like.
        let named = open || self.cycle.is_multiple_of(KEEP_REF_EVERY);
        let branch = if named {
            format!("refs/heads/topic/{:06}", self.cycle)
        } else {
            SCRATCH_BRANCH.to_owned()
        };

        let mut topic = base;
        let mut content = String::new();
        for step in 1..=TOPIC_RUN {
            content = format!("topic {} step {step}\n", self.cycle);
            let message = format!("topic/{:06}: step {step}\n", self.cycle);
            let (identity, message) = self.dressed(message);
            topic = self.stream.commit_by(
                &identity,
                branch.as_bytes(),
                message.as_bytes(),
                &[topic],
                &[(b"topic.txt", content.as_bytes())],
            );
            self.counted();
        }

        if !open {
            let message = format!("Merge branch 'topic/{:06}'\n", self.cycle);
            // `fast-import` starts a merge from the *first* parent's tree and
            // applies only what the stream spells out, so the topic's file
            // has to be restated here or the merge would undo it.
            let (identity, message) = self.dressed(message);
            self.tip = Some(self.stream.commit_by(
                &identity,
                MAIN,
                message.as_bytes(),
                &[base, topic],
                &[(b"topic.txt", content.as_bytes())],
            ));
            self.counted();
            self.tag();
        }

        self.cycle += 1;
        self.maybe_drain(out)
    }

    /// The identity and message the next commit carries: `subject` and the
    /// fixture identity for small commits; for realistic ones, a body and a
    /// sign-off added to the subject and one of a thousand identities, both
    /// picked from the commit's number so the fixture is the same every build.
    fn dressed(&self, subject: String) -> (String, String) {
        match self.size {
            CommitSize::Small => (crate::stream::IDENTITY.to_owned(), subject),
            CommitSize::Realistic => {
                let n = self.done + 1;
                let identity = realistic_identity(n);
                let message = realistic_message(n, &subject, &identity);
                (identity, message)
            }
        }
    }

    /// Records one emitted commit.
    fn counted(&mut self) {
        self.done += 1;
        self.left -= 1;
        self.buffered += 1;
    }

    /// A tag on the trunk, if this commit is one of the ones that gets one.
    fn tag(&mut self) {
        let Some(tip) = self.tip else { return };
        if self.done.is_multiple_of(ANNOTATED_TAG_EVERY) {
            let name = format!("release/{:07}", self.done);
            let message = format!("Release at commit {}\n", self.done);
            self.stream
                .annotated_tag(name.as_bytes(), tip, message.as_bytes());
        } else if self.done.is_multiple_of(LIGHT_TAG_EVERY) {
            let name = format!("refs/tags/checkpoint/{:07}", self.done);
            self.stream.point(name.as_bytes(), tip);
        }
    }

    /// Empties the buffer into the file every so often, so that the
    /// generator's memory does not grow with the size of the fixture.
    fn maybe_drain(&mut self, out: &mut impl io::Write) -> Result<(), io::Error> {
        if self.buffered >= DRAIN_EVERY {
            self.buffered = 0;
            self.stream.drain_into(out)?;
        }
        Ok(())
    }
}

/// Body lines of a realistic message, picked by a commit's number. At 64
/// bytes a line this is a mean of ~500 bytes (median ~350, p90 ~770) before
/// the subject and sign-off: with the headers, git/git's commit sizes.
const BODY_LINES: [u32; 10] = [2, 3, 4, 5, 5, 6, 7, 9, 12, 24];

/// Identities a realistic fixture spreads its commits over.
const IDENTITIES: u32 = 1000;

/// A number that looks unrelated to `n` but is always the same for it
/// (Knuth's multiplicative hash), so neighbouring commits differ in size.
fn scatter(n: u32) -> u32 {
    n.wrapping_mul(2_654_435_761)
}

/// About 35 bytes, like git/git's mean identity.
fn realistic_identity(n: u32) -> String {
    let who = (scatter(n) >> 12) % IDENTITIES;
    format!("Author {who:04} <a{who:04}@example.invalid>")
}

/// `subject`, a blank line, a wrapped body and a `Signed-off-by` trailer.
fn realistic_message(n: u32, subject: &str, identity: &str) -> String {
    const LINE: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do.\n";
    let lines = BODY_LINES[(scatter(n) % 10) as usize];
    let mut message = String::with_capacity(subject.len() + LINE.len() * lines as usize + 64);
    message.push_str(subject);
    message.push('\n');
    for _ in 0..lines {
        message.push_str(LINE);
    }
    message.push_str("\nSigned-off-by: ");
    message.push_str(identity);
    message.push('\n');
    message
}
